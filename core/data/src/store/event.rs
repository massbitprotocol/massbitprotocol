use crate::store::entity::{EntityChange, EntityModification, SubscriptionFilter};
use massbit_common::prelude::slog::{trace, Logger};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::iter::FromIterator;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::store::QueryStore;
use core::time::Duration;
use futures::stream::poll_fn;
use futures::{Async, Poll, Stream};
use futures_util::future::FutureExt;
use futures_util::TryFutureExt;
use massbit_common::prelude::tokio;

#[derive(Clone, Debug, Serialize, Deserialize)]
/// The store emits `StoreEvents` to indicate that some entities have changed.
/// For block-related data, at most one `StoreEvent` is emitted for each block
/// that is processed. The `changes` vector contains the details of what changes
/// were made, and to which entity.
///
/// Since the 'subgraph of subgraphs' is special, and not directly related to
/// any specific blocks, `StoreEvents` for it are generated as soon as they are
/// written to the store.
pub struct StoreEvent {
    // The tag is only there to make it easier to track StoreEvents in the
    // logs as they flow through the system
    pub tag: usize,
    pub changes: HashSet<EntityChange>,
}

impl<'a> FromIterator<&'a EntityModification> for StoreEvent {
    fn from_iter<I: IntoIterator<Item = &'a EntityModification>>(mods: I) -> Self {
        let changes: Vec<_> = mods
            .into_iter()
            .map(|op| {
                use self::EntityModification::*;
                match op {
                    Insert { key, .. } | Overwrite { key, .. } | Remove { key } => {
                        EntityChange::for_data(key.clone())
                    }
                }
            })
            .collect();
        StoreEvent::new(changes)
    }
}

impl StoreEvent {
    pub fn new(changes: Vec<EntityChange>) -> StoreEvent {
        static NEXT_TAG: AtomicUsize = AtomicUsize::new(0);

        let tag = NEXT_TAG.fetch_add(1, Ordering::Relaxed);
        let changes = changes.into_iter().collect();
        StoreEvent { tag, changes }
    }

    /// Extend `ev1` with `ev2`. If `ev1` is `None`, just set it to `ev2`
    fn accumulate(logger: &Logger, ev1: &mut Option<StoreEvent>, ev2: StoreEvent) {
        if let Some(e) = ev1 {
            trace!(logger, "Adding changes to event";
                           "from" => ev2.tag, "to" => e.tag);
            e.changes.extend(ev2.changes);
        } else {
            *ev1 = Some(ev2);
        }
    }

    pub fn extend(mut self, other: StoreEvent) -> Self {
        self.changes.extend(other.changes);
        self
    }
}

impl fmt::Display for StoreEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "StoreEvent[{}](changes: {})",
            self.tag,
            self.changes.len()
        )
    }
}

impl PartialEq for StoreEvent {
    fn eq(&self, other: &StoreEvent) -> bool {
        // Ignore tag for equality
        self.changes == other.changes
    }
}

/// A `StoreEventStream` produces the `StoreEvents`. Various filters can be applied
/// to it to reduce which and how many events are delivered by the stream.
pub struct StoreEventStream<S> {
    source: S,
}

/// A boxed `StoreEventStream`
pub type StoreEventStreamBox =
    StoreEventStream<Box<dyn Stream<Item = Arc<StoreEvent>, Error = ()> + Send>>;

impl<S> Stream for StoreEventStream<S>
where
    S: Stream<Item = Arc<StoreEvent>, Error = ()> + Send,
{
    type Item = Arc<StoreEvent>;
    type Error = ();

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        self.source.poll()
    }
}

impl<S> StoreEventStream<S>
where
    S: Stream<Item = Arc<StoreEvent>, Error = ()> + Send + 'static,
{
    // Create a new `StoreEventStream` from another such stream
    pub fn new(source: S) -> Self {
        StoreEventStream { source }
    }

    /// Filter a `StoreEventStream` by subgraph and entity. Only events that have
    /// at least one change to one of the given (subgraph, entity) combinations
    /// will be delivered by the filtered stream.
    pub fn filter_by_entities(self, filters: Vec<SubscriptionFilter>) -> StoreEventStreamBox {
        let source = self.source.filter(move |event| {
            event
                .changes
                .iter()
                .any(|change| filters.iter().any(|filter| filter.matches(change)))
        });

        StoreEventStream::new(Box::new(source))
    }

    /// Reduce the frequency with which events are generated while a
    /// subgraph deployment is syncing. While the given `deployment` is not
    /// synced yet, events from `source` are reported at most every
    /// `interval`. At the same time, no event is held for longer than
    /// `interval`. The `StoreEvents` that arrive during an interval appear
    /// on the returned stream as a single `StoreEvent`; the events are
    /// combined by using the maximum of all sources and the concatenation
    /// of the changes of the `StoreEvents` received during the interval.
    pub async fn throttle_while_syncing(
        self,
        logger: &Logger,
        store: Arc<dyn QueryStore>,
        interval: Duration,
    ) -> StoreEventStreamBox {
        // Check whether a deployment is marked as synced in the store. Note that in the moment a
        // subgraph becomes synced any existing subscriptions will continue to be throttled since
        // this is not re-checked.
        let synced = store.is_deployment_synced().await.unwrap_or(false);

        let mut pending_event: Option<StoreEvent> = None;
        let mut source = self.source.fuse();
        let mut had_err = false;
        let mut delay = tokio::time::sleep(interval).unit_error().boxed().compat();
        let logger = logger.clone();

        let source = Box::new(poll_fn(move || -> Poll<Option<Arc<StoreEvent>>, ()> {
            if had_err {
                // We had an error the last time through, but returned the pending
                // event first. Indicate the error now
                had_err = false;
                return Err(());
            }

            if synced {
                return source.poll();
            }

            // Check if interval has passed since the last time we sent something.
            // If it has, start a new delay timer
            let should_send = match futures::future::Future::poll(&mut delay) {
                Ok(Async::NotReady) => false,
                // Timer errors are harmless. Treat them as if the timer had
                // become ready.
                Ok(Async::Ready(())) | Err(_) => {
                    delay = tokio::time::sleep(interval).unit_error().boxed().compat();
                    true
                }
            };

            // Get as many events as we can off of the source stream
            loop {
                match source.poll() {
                    Ok(Async::NotReady) => {
                        if should_send && pending_event.is_some() {
                            let event = pending_event.take().map(|event| Arc::new(event));
                            return Ok(Async::Ready(event));
                        } else {
                            return Ok(Async::NotReady);
                        }
                    }
                    Ok(Async::Ready(None)) => {
                        let event = pending_event.take().map(|event| Arc::new(event));
                        return Ok(Async::Ready(event));
                    }
                    Ok(Async::Ready(Some(event))) => {
                        StoreEvent::accumulate(&logger, &mut pending_event, (*event).clone());
                    }
                    Err(()) => {
                        // Before we report the error, deliver what we have accumulated so far.
                        // We will report the error the next time poll() is called
                        if pending_event.is_some() {
                            had_err = true;
                            let event = pending_event.take().map(|event| Arc::new(event));
                            return Ok(Async::Ready(event));
                        } else {
                            return Err(());
                        }
                    }
                };
            }
        }));
        StoreEventStream::new(source)
    }
}

use futures03::channel::oneshot;
use futures03::{future::Fuse, Future, FutureExt, Stream};
use std::fmt::{Debug, Display};
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::Context;
use std::time::Duration;
use tokio::macros::support::Poll;

/// A cancelable stream or future.
///
/// Created by calling `cancelable` extension method.
/// Can be canceled through the corresponding `CancelGuard`.
pub struct Cancelable<T, C> {
    inner: T,
    cancel_receiver: Fuse<oneshot::Receiver<()>>,
    on_cancel: C,
}

impl<F: Future + Unpin, C: Fn() -> F::Output + Unpin> Future for Cancelable<F, C> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Error if the future was canceled by dropping the sender.
        // `canceled` is fused so we may ignore `Ok`s.
        match self.cancel_receiver.poll_unpin(cx) {
            Poll::Ready(Ok(_)) => unreachable!(),
            Poll::Ready(Err(_)) => Poll::Ready((self.on_cancel)()),
            Poll::Pending => Pin::new(&mut self.inner).poll(cx),
        }
    }
}

/// A `CancelGuard` or `SharedCancelGuard`.
pub trait Canceler {
    /// Adds `cancel_sender` to the set being guarded.
    /// Avoid calling directly and prefer using `cancelable`.
    fn add_cancel_sender(&self, cancel_sender: oneshot::Sender<()>);
}

pub trait FutureExtension: Future + Sized {
    /// When `cancel` is called on a `CancelGuard` or it is dropped,
    /// `Cancelable` receives an error.
    ///
    /// `on_cancel` is called to make an error value upon cancelation.
    fn cancelable<C: Fn() -> Self::Output>(
        self,
        guard: &impl Canceler,
        on_cancel: C,
    ) -> Cancelable<Self, C>;

    fn timeout(self, dur: Duration) -> tokio::time::Timeout<Self>;
}

impl<F: Future> FutureExtension for F {
    fn cancelable<C: Fn() -> F::Output>(
        self,
        guard: &impl Canceler,
        on_cancel: C,
    ) -> Cancelable<Self, C> {
        let (canceler, cancel_receiver) = oneshot::channel();
        guard.add_cancel_sender(canceler);
        Cancelable {
            inner: self,
            cancel_receiver: cancel_receiver.fuse(),
            on_cancel,
        }
    }

    fn timeout(self, dur: Duration) -> tokio::time::Timeout<Self> {
        tokio::time::timeout(dur, self)
    }
}

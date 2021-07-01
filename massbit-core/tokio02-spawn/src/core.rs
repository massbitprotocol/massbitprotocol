use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, Future, TryFutureExt};
use futures::future::FutureExt;
use std::fmt::Debug;
use std::panic::AssertUnwindSafe;

// This is a hack required because the json-rpc crate is not updated to tokio 0.2.
// We should watch the `jsonrpsee` crate and switch to that once it's ready.
pub async fn tokio02_spawn<I: Send + 'static, ER: Send + 'static>(
    mut task_sink: mpsc::Sender<Box<dyn std::future::Future<Output = ()> + Send + Unpin>>,
    future: impl std::future::Future<Output = Result<I, ER>> + Send + Unpin + 'static,
) -> Result<I, ER>
    where
        I: Debug,
        ER: Debug,
{
    let (return_sender, return_receiver) = oneshot::channel();
    task_sink
        .send(Box::new(future.map(move |res| {
            return_sender.send(res).expect("`return_receiver` dropped");
        })))
        .await
        .expect("task receiver dropped");
    return_receiver.await.expect("`return_sender` dropped")
}

pub fn abort_on_panic<T: Send + 'static>(
    f: impl Future<Output = T> + Send + 'static,
) -> impl Future<Output = T> {
    // We're crashing, unwind safety doesn't matter.
    AssertUnwindSafe(f).catch_unwind().unwrap_or_else(|_| {
        println!("Panic in tokio task, aborting!");
        std::process::abort()
    })
}
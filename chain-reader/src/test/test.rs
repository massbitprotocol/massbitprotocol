use broadcaster::BroadcastChannel;
use futures_util::StreamExt;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // let mut chan = BroadcastChannel::new();
    // chan.send(&5i32).await?;
    // assert_eq!(chan.next().await, Some(5));
    //
    // let mut chan2 = chan.clone();
    // chan2.send(&6i32).await?;
    // assert_eq!(chan.next().await, Some(6));
    // assert_eq!(chan2.next().await, Some(6));
    Ok(())
}
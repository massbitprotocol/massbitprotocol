use futures03::{FutureExt, Stream, StreamExt};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::firehose::endpoints::FirehoseEndpoint;
use crate::prelude::*;

use super::block_stream::{BlockStream, BlockStreamEvent, FirehoseMapper};
use super::Blockchain;
use crate::firehose::bstream;

pub struct FirehoseBlockStreamContext<C, F>
where
    C: Blockchain,
    F: FirehoseMapper<C>,
{
    mapper: Arc<F>,
    network: String,
    filter: Arc<C::TriggerFilter>,
    start_block: BlockNumber,
    logger: Logger,
}

impl<C: Blockchain, F: FirehoseMapper<C>> Clone for FirehoseBlockStreamContext<C, F> {
    fn clone(&self) -> Self {
        Self {
            mapper: self.mapper.clone(),
            network: self.network.clone(),
            filter: self.filter.clone(),
            start_block: self.start_block.clone(),
            logger: self.logger.clone(),
        }
    }
}

enum BlockStreamState {
    Disconnected,
    Connecting(
        Pin<
            Box<
                dyn futures03::Future<
                    Output = Result<tonic::Streaming<bstream::BlockResponse>, anyhow::Error>,
                >,
            >,
        >,
    ),
    Connected(tonic::Streaming<bstream::BlockResponse>),
}

pub struct FirehoseBlockStream<C: Blockchain, F: FirehoseMapper<C>> {
    endpoint: Arc<FirehoseEndpoint>,
    state: BlockStreamState,
    ctx: FirehoseBlockStreamContext<C, F>,
    connection_attempts: u64,
}

impl<C, F> FirehoseBlockStream<C, F>
where
    C: Blockchain,
    F: FirehoseMapper<C>,
{
    pub fn new(
        endpoint: Arc<FirehoseEndpoint>,
        mapper: Arc<F>,
        network: String,
        filter: Arc<C::TriggerFilter>,
        start_block: BlockNumber,
        logger: Logger,
    ) -> Self {
        FirehoseBlockStream {
            endpoint,
            state: BlockStreamState::Disconnected,
            ctx: FirehoseBlockStreamContext {
                mapper,
                network,
                logger,
                filter,
                start_block,
            },
            connection_attempts: 0,
        }
    }
}

impl<C: Blockchain, F: FirehoseMapper<C>> BlockStream<C> for FirehoseBlockStream<C, F> {}

impl<C: Blockchain, F: FirehoseMapper<C>> Stream for FirehoseBlockStream<C, F> {
    type Item = Result<BlockStreamEvent<C>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match &mut self.state {
                BlockStreamState::Disconnected => {
                    info!(
                        self.ctx.logger,
                        "Blockstream disconnected, (re-)connecting"; "endpoint uri" => format_args!("{}", self.endpoint),
                    );

                    let filter_bytes = serde_json::to_vec(&self.ctx.filter)?;
                    let future = self.endpoint.clone().stream_blocks(bstream::BlockRequest {
                        start_block_number: Some(self.ctx.start_block as u64),
                        chain_type: bstream::ChainType::Ethereum as i32,
                        network: self.ctx.network.clone(),
                        filter: filter_bytes,
                    });
                    let mut stream_connection = Box::pin(future);

                    match stream_connection.poll_unpin(cx) {
                        Poll::Ready(Ok(streaming)) => {
                            self.state = BlockStreamState::Connected(streaming);
                            self.connection_attempts = 0;
                            info!(self.ctx.logger, "Blockstream connected");

                            // Re-loop to next state
                            continue;
                        }

                        Poll::Ready(Err(e)) => {
                            error!(self.ctx.logger, "Unable to connect to endpoint {}", e);
                            return self.schedule_error_retry(cx);
                        }

                        Poll::Pending => {
                            trace!(
                                self.ctx.logger,
                                "Connection is still pending when being created"
                            );
                            self.state = BlockStreamState::Connecting(stream_connection);
                            return Poll::Pending;
                        }
                    }
                }

                BlockStreamState::Connecting(stream_connection) => {
                    match stream_connection.poll_unpin(cx) {
                        Poll::Ready(Ok(streaming)) => {
                            self.state = BlockStreamState::Connected(streaming);
                            info!(self.ctx.logger, "Blockstream connected");

                            // Re-loop to next state
                            continue;
                        }

                        Poll::Ready(Err(e)) => {
                            error!(self.ctx.logger, "Unable to connect to endpoint {}", e);
                            return self.schedule_error_retry(cx);
                        }

                        Poll::Pending => {
                            trace!(
                                self.ctx.logger,
                                "Connection is still pending when being wake up"
                            );

                            return Poll::Pending;
                        }
                    }
                }

                BlockStreamState::Connected(streaming) => match streaming.poll_next_unpin(cx) {
                    Poll::Ready(Some(Ok(response))) => {
                        match self
                            .ctx
                            .mapper
                            .to_block_stream_event(&self.ctx.logger, &response)
                        {
                            Ok(event) => {
                                return Poll::Ready(Some(Ok(event)));
                            }
                            Err(e) => {
                                error!(
                                    self.ctx.logger,
                                    "Mapping block to BlockStreamEvent failed {}", e
                                );
                                self.state = BlockStreamState::Disconnected;

                                return self.schedule_error_retry(cx);
                            }
                        }
                    }

                    Poll::Ready(Some(Err(e))) => {
                        error!(self.ctx.logger, "Stream disconnected from endpoint {}", e);
                        self.state = BlockStreamState::Disconnected;

                        return self.schedule_error_retry(cx);
                    }

                    Poll::Ready(None) => {
                        error!(self.ctx.logger, "Stream has terminated blocks range, we expect never ending stream right now");
                        self.state = BlockStreamState::Disconnected;

                        return self.schedule_error_retry(cx);
                    }

                    Poll::Pending => {
                        trace!(
                            self.ctx.logger,
                            "Stream is pending, no item available yet will being wake up"
                        );

                        return Poll::Pending;
                    }
                },
            }
        }
    }
}

impl<C: Blockchain, F: FirehoseMapper<C>> FirehoseBlockStream<C, F> {
    /// Schedule a delayed function that will wake us later in time. This implementation
    /// uses an exponential backoff strategy to retry with incremental longer delays.
    fn schedule_error_retry<T>(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.connection_attempts += 1;
        let wait_duration = wait_duration(self.connection_attempts);

        let waker = cx.waker().clone();
        tokio::spawn(async move {
            tokio::time::sleep(wait_duration).await;
            waker.wake();
        });

        Poll::Pending
    }
}

fn wait_duration(attempt_number: u64) -> Duration {
    let pow = if attempt_number > 5 {
        5
    } else {
        attempt_number
    };

    Duration::from_secs(2 << pow)
}

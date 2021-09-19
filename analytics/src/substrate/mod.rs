use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};
use log::{debug, info, warn};
use std::time::Instant;
use massbit_chain_substrate::data_type::{SubstrateBlock, SubstrateEventRecord, decode, get_extrinsics_from_block};

pub async fn process_substrate_block(mut client: StreamoutClient<Channel>) ->  Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let get_blocks_request = GetBlocksRequest {
        start_block_number: 0,
        end_block_number: 1,
        chain_type: ChainType::Substrate as i32,
        network: String::from(""),
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    log::info!("Starting read blocks from stream...");
    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        let now = Instant::now();
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let block: SubstrateBlock = decode(&mut data.payload).unwrap();
                info!("Received BLOCK: {:?}", &block.block.header.number);
                let extrinsics = get_extrinsics_from_block(&block);
                for extrinsic in extrinsics {
                    //info!("Recieved EXTRINSIC: {:?}", extrinsic);
                    let string_extrinsic = format!("Recieved EXTRINSIC:{:?}", extrinsic);
                    info!("{}", string_extrinsic);
                }
            }
            Some(DataType::Event) => {
                let event: Vec<SubstrateEventRecord> = decode(&mut data.payload).unwrap();
                info!("Received Event: {:?}", event);
            }

            _ => {
                warn!("Not support data type: {:?}", &data.data_type);
            }
        }
        let elapsed = now.elapsed();
        debug!("Elapsed processing solana block: {:.2?}", elapsed);
    };
    Ok(())
}
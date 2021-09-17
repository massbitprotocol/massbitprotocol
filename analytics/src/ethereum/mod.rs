use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};
use massbit_chain_ethereum::data_type::{decode as ethereum_decode, get_events, EthereumBlock};

pub fn process_ethereum_block(client: &StreamoutClient<Channel>, network: &String) ->  Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let get_blocks_request = GetBlocksRequest {
        start_block_number: 0,
        end_block_number: 1,
        chain_type: chain_type as i32,
        network,
    };
    let mut stream = client
        .list_blocks(Request::new(get_blocks_request))
        .await?
        .into_inner();

    log::info!("Starting read blocks from stream...");
    while let Some(data) = stream.message().await? {
        let mut data = data as GenericDataProto;
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let block: EthereumBlock = ethereum_decode(&mut data.payload).unwrap();
                info!(
                        "Recieved ETHREUM BLOCK with Block number: {}",
                        &block.block.number.unwrap().as_u64()
                    );

            }
            _ => {
                warn!("Not support this type in Ethereum");
            }
        },
    }
    Ok(())
}
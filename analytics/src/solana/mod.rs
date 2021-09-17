use crate::stream_mod::{
    streamout_client::StreamoutClient, ChainType, DataType, GenericDataProto, GetBlocksRequest,
};
#[allow(unused_imports)]
use tonic::{
    transport::{Channel, Server},
    Request, Response, Status,
};

pub fn process_solana_block(client: &StreamoutClient<Channel>) ->  Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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
        let now = Instant::now();
        match DataType::from_i32(data.data_type) {
            Some(DataType::Block) => {
                let encoded_block: SolanaEncodedBlock =
                    solana_decode(&mut data.payload).unwrap();
                // Decode
                let block = convert_solana_encoded_block_to_solana_block(encoded_block);
                let mut print_flag = true;
                for origin_transaction in block.clone().block.transactions {
                    let log_messages = origin_transaction
                        .clone()
                        .meta
                        .unwrap()
                        .log_messages
                        .clone();
                    let transaction = SolanaTransaction {
                        block_number: ((&block).block.block_height.unwrap() as u32),
                        transaction: origin_transaction.clone(),
                        log_messages: log_messages.clone(),
                        success: false,
                    };
                    let log_messages = SolanaLogMessages {
                        block_number: ((&block).block.block_height.unwrap() as u32),
                        log_messages: log_messages.clone(),
                        transaction: origin_transaction.clone(),
                    };

                    // Print first data only bc it too many.
                    if print_flag {
                        info!("Recieved SOLANA TRANSACTION with Block number: {:?}, trainsation: {:?}", &transaction.block_number, &transaction.transaction.transaction.signatures);
                        info!("Recieved SOLANA LOG_MESSAGES with Block number: {:?}, log_messages: {:?}", &log_messages.block_number, &log_messages.log_messages.unwrap().get(0));

                        print_flag = false;
                    }
                }
            }
            _ => {
                warn!("Not support this type in Solana");
            }
        }
        let elapsed = now.elapsed();
        debug!("Elapsed processing solana block: {:.2?}", elapsed);
    }
}
use crate::models::*;
use massbit_chain_solana::data_type as types;
use std::error::Error;
use uuid::Uuid;


pub fn handle_transaction(transaction: &types::SolanaTransaction) -> Result<(), Box<dyn Error>> {
    println!("[SO File] [Test Insert data] Received Transaction Messages");
    let transaction_uuid = Uuid::new_v4();
    let block_uuid = Uuid::new_v4();
    let transaction_id = transaction_uuid.to_simple().to_string();
    let block_id = block_uuid.to_simple().to_string();
    println!("transaction_uuid: {}", transaction_id);
    println!("transaction_uuid: {}", block_id);
    let transaction_ts = Transaction {
        id: transaction_id,
        signature: format!("{:?}", transaction.transaction.transaction.signatures),
        timestamp: Default::default(),
        fee: Default::default(),
        block: block_id.clone(),
        block_number: Default::default(),
        success: true,

    };
    let block_ts = Block {
        id: block_id,
        block_number: transaction.block_number as i64,
        block_hash: Default::default(),
        sum_fee: Default::default(),
        transaction_number: Default::default(),
        success_rate: Default::default()

    };
    block_ts.save();
    transaction_ts.save();

    Ok(())
}

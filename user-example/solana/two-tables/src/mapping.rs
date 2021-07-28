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

        // pub id: String,
        // pub signature: String,
        // pub timestamp: i64,
        // pub fee: i64,
        // pub block: String,
        // pub block_number: i64,
        // pub success: bool, // Support bool?
    };
    let block_ts = Block {
        id: block_id,
        block_number: transaction.block_number as i64,
        block_hash: Default::default(),
        sum_fee: Default::default(),
        transaction_number: Default::default(),
        success_rate: Default::default()

        // pub id: String,
        // pub block_number: i64,
        // pub block_hash: String,
        // pub sum_fee: i64,
        // pub transaction_number: i64,
        // pub success_rate: i64
    };
    block_ts.save();
    // transaction_ts.save();

    Ok(())
}

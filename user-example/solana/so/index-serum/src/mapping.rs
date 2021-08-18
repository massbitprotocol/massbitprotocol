use crate::models::*;
use massbit_chain_substrate::data_type as substrate_types;
use massbit_chain_solana::data_type as solana_types;
use std::error::Error;
use uuid::Uuid;

fn is_serum(pubkeys: &Vec<solana_types::Pubkey>) -> bool {
    pubkeys.iter().any(|&key| key.to_string()=="9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin")
}

pub fn handle_block(block: &solana_types::SolanaBlock) -> Result<(), Box<dyn Error>> {
    // Create ID
    let block_id = Uuid::new_v4().to_simple().to_string();
    // Create Block
    let block_ts = SerumBlock {
        id: block_id.clone(),
        block_number: block.block.block_height.unwrap() as i64,
        block_hash: block.block.blockhash.to_string(),
        sum_fee: Default::default(),
        transaction_number: block.block.transactions.len() as i64,
        success_rate: Default::default()
    };
    block_ts.save();
    // Create transaction
    for transaction in &block.block.transactions{
        if !is_serum(&transaction.transaction.message.account_keys){
            continue;
        }
        let transaction_id = Uuid::new_v4().to_simple().to_string();

        let meta = transaction.meta.clone().unwrap();
        let transaction_ts = SerumTransaction {
            id: transaction_id.clone(),
            signatures: transaction.transaction.signatures.iter()
                .map(|sign| sign.to_string())
                .collect(),
            timestamp: block.timestamp,
            fee: meta.fee as i64,
            block: block_id.clone(),
            block_number: block.block.block_height.unwrap() as i64,
            // Todo: get success
            success: true,
        };
        transaction_ts.save();

        // Create transaction account
        for (index,account) in transaction.transaction.message.account_keys.iter().enumerate(){
            let transaction_account_id = Uuid::new_v4().to_simple().to_string();
            let transaction_account = SerumTransactionAccount {
                id: transaction_account_id,
                pub_key: account.to_string(),
                pos_balance: meta.post_balances[index] as i64,
                change_balance: (meta.post_balances[index] as i64 - meta.pre_balances[index] as i64),
                // Todo: get is_program
                is_program: false,
                transaction_own: transaction_id.clone(),
                inner_account_index: index as i64,
            };
            transaction_account.save();
        }

        // Create Transaction Instruction
        for instruction in &meta.inner_instructions.unwrap() {
            let instruction_detail_id = Uuid::new_v4().to_simple().to_string();
            let instruction_detail_ts = SerumInstructionDetail {
                id: instruction_detail_id.clone(),
                // Todo: get name and is_decoded
                name: format!("{:?}",instruction.instructions),
                is_decoded: false
            };
            instruction_detail_ts.save();

            let instruction_id = Uuid::new_v4().to_simple().to_string();
            let instruction_ts = SerumTransactionInstruction {
                id: instruction_id,
                transaction_own: transaction_id.clone(),
                inner_account_index: instruction.instructions[0].program_id_index as i64,
                instruction_detail: instruction_detail_id
            };
            instruction_ts.save();

        }
    }

    Ok(())
}
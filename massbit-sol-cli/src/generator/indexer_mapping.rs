pub const INDEXER_MAPPING: &str = r#"
use crate::generated::handler::Handler;
use crate::ADDRESS;
use massbit_solana_sdk::types::SolanaBlock;
use solana_transaction_status::TransactionWithStatusMeta;

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    for (tx_ind, tran) in block.block.transactions.iter().enumerate() {
        if tran
            .transaction
            .message
            .account_keys
            .iter()
            .any(|key| key.to_string().as_str() == ADDRESS)
        {
            parse_instructions(block, tran, tx_ind);
        }
    }
    Ok(())
}
fn parse_instructions(block: &SolanaBlock, tran: &TransactionWithStatusMeta, tx_ind: usize) {
    for (ind, inst) in tran.transaction.message.instructions.iter().enumerate() {
        let program_key = inst.program_id(tran.transaction.message.account_keys.as_slice());
        if program_key.to_string().as_str() == ADDRESS {
            let mut accounts = Vec::default();
            let mut work = |unique_ind: usize, acc_ind: usize| {
                if let Some(key) = tran.transaction.message.account_keys.get(acc_ind) {
                    accounts.push(key.clone());
                };
                Ok(())
            };
            inst.visit_each_account(&mut work);
            // if let Some(account_infos) = SOLANA_CLIENT
            //     .get_multiple_accounts_with_config(
            //         accounts.as_slice(),
            //         RpcAccountInfoConfig {
            //             encoding: Some(UiAccountEncoding::JsonParsed),
            //             commitment: None,
            //             data_slice: None,
            //         },
            //     )
            //     .map(|res| {
            //         res.value
            //             .into_iter()
            //             .filter_map(|elm| elm)
            //             .collect::<Vec<Account>>()
            //     })
            //     .ok()
            // {
            //println!("account_infos {:?}", &account_infos);
            let handler = Handler {};
            // Fixme: Get account_infos from chain take a lot of time. For now, use empty vector.
            handler.process(block, tran, program_key, &accounts, inst.data.as_slice());
            // }
        }
    }
}

"#;

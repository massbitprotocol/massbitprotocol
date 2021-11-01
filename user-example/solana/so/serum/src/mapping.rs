//use super::serum_dex::{instruction::MarketInstruction, state::State};
use crate::models::*;
use crate::SOLANA_CLIENT;
use massbit_chain_solana::data_type::{SolanaBlock, SolanaLogMessages, SolanaTransaction};
use massbit_chain_solana::helper::get_account_info;
use massbit_chain_solana::{get_mint_account, get_owner_account};
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_client::rpc_response::RpcResult;
use solana_client::{client_error::Result as ClientResult, rpc_request::RpcRequest};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::CompiledInstruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use solana_transaction_status::{parse_instruction, ConfirmedBlock, TransactionWithStatusMeta};
use uuid::Uuid;
use crate::generated::instruction::MarketInstruction;
use crate::generated::handler::SerumHandler;

const ADDRESS: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";

// fn is_match(pubkeys: &Vec<Pubkey>) -> bool {
//     pubkeys.iter().any(|&key| key.to_string() == ADDRESS)
// }

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    for (tx_ind, tran) in block.block.transactions.iter().enumerate() {
        if tran
            .transaction
            .message
            .account_keys
            .iter()
            .any(|key| key.to_string().as_str() == ADDRESS)
        {
            let entities = parse_instructions(&block.block, tran, tx_ind);
        }
    }
    Ok(())
}
fn parse_instructions(block: &ConfirmedBlock, tran: &TransactionWithStatusMeta, tx_ind: usize) {
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
            if let Some(account_infos) = SOLANA_CLIENT
                .get_multiple_accounts_with_config(
                    accounts.as_slice(),
                    RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::JsonParsed),
                        commitment: None,
                        data_slice: None,
                    },
                ).map(|res| res.value.into_iter().filter_map(|elm| elm).collect::<Vec<Account>>()).ok() {
                //println!("{:?}", &account_infos);
                let serum_handler = SerumHandler{};
                serum_handler.process(program_key, account_infos.as_slice(), inst.data.as_slice());
            }
        }
    }
    let id = Uuid::new_v4().to_simple().to_string();
}

//
// fn process_admin_instruction(
//     block: &ConfirmedBlock,
//     tran: &TransactionWithStatusMeta,
//     instruction: &AdminInstruction,
// ) {
//     match instruction {
//         AdminInstruction::RampA(ramp_a) => {}
//         AdminInstruction::StopRampA => {}
//         AdminInstruction::Pause => {}
//         AdminInstruction::Unpause => {}
//         AdminInstruction::SetFeeAccount => {}
//         AdminInstruction::ApplyNewAdmin => {}
//         AdminInstruction::CommitNewAdmin => {}
//         AdminInstruction::SetNewFees(fees) => {}
//     }
// }
//
// fn process_swap_instruction(
//     block: &ConfirmedBlock,
//     tran: &TransactionWithStatusMeta,
//     tx_ind: usize,
//     inst: &CompiledInstruction,
//     inst_ind: usize,
//     instruction: &SwapInstruction,
// ) {
//     match instruction {
//         SwapInstruction::Initialize(init) => {
//             //println!("{:?}", tran.transaction.message.account_keys.as_slice());
//             let tx_hash = tran
//                 .transaction
//                 .signatures
//                 .get(0)
//                 .and_then(|sig| Some(sig.to_string()))
//                 .unwrap_or_default();
//             let entity = SaberInit {
//                 block_slot: block.parent_slot as i64 + 1,
//                 parent_slot: block.parent_slot as i64,
//                 tx_index: tx_ind as i64,
//                 tx_hash,
//                 instruction_index: inst_ind as i64,
//                 block_time: block.block_time.unwrap_or_default(),
//                 id: tran
//                     .transaction
//                     .message
//                     .account_keys
//                     .get(
//                         inst.accounts
//                             .get(0)
//                             .and_then(|ind| Some(*ind as usize))
//                             .unwrap(),
//                     )
//                     .and_then(|key| Some(key.to_string()))
//                     .unwrap_or_default(),
//                 nonce: init.nonce as i64,
//                 amp_factor: init.amp_factor as i64,
//                 admin_trade_fee_numerator: init.fees.admin_trade_fee_numerator as i64,
//                 admin_trade_fee_denominator: init.fees.admin_trade_fee_denominator as i64,
//                 admin_withdraw_fee_numerator: init.fees.admin_withdraw_fee_numerator as i64,
//                 admin_withdraw_fee_denominator: init.fees.admin_withdraw_fee_denominator as i64,
//                 trade_fee_numerator: init.fees.trade_fee_numerator as i64,
//                 trade_fee_denominator: init.fees.trade_fee_denominator as i64,
//                 withdraw_fee_numerator: init.fees.withdraw_fee_numerator as i64,
//                 withdraw_fee_denominator: init.fees.withdraw_fee_denominator as i64,
//             };
//             entity.save();
//         }
//         SwapInstruction::Swap(swap) => {
//             let swap_entity = create_swap_entity(block, tran, tx_ind, inst, inst_ind, swap);
//             swap_entity.save();
//         }
//         SwapInstruction::Deposit(deposit) => {
//             //println!("{:?}", tran.transaction.message.account_keys.as_slice());
//             let deposit_entity =
//                 create_deposit_entity(block, tran, tx_ind, inst, inst_ind, deposit);
//             deposit_entity.save();
//         }
//         SwapInstruction::Withdraw(withdraw) => {
//             let withdraw_entity =
//                 create_withdraw_entity(block, tran, tx_ind, inst, inst_ind, withdraw);
//             withdraw_entity.save()
//         }
//         SwapInstruction::WithdrawOne(with_draw_one) => {
//             let with_draw_one_entity =
//                 create_withdraw_one_entity(block, tran, tx_ind, inst, inst_ind, with_draw_one);
//             with_draw_one_entity.save();
//         }
//     }
// }
//
// fn create_swap_entity(
//     block: &ConfirmedBlock,
//     tran: &TransactionWithStatusMeta,
//     tx_ind: usize,
//     inst: &CompiledInstruction,
//     inst_ind: usize,
//     swap: &SwapData,
// ) -> SaberSwap {
//     let tx_hash = tran
//         .transaction
//         .signatures
//         .get(0)
//         .and_then(|sig| Some(sig.to_string()))
//         .unwrap_or_default();
//     let program_key = inst.program_id(tran.transaction.message.account_keys.as_slice());
//     let source_account = tran.transaction.message.account_keys.get(
//         inst.accounts
//             .get(3)
//             .and_then(|ind| Some(*ind as usize))
//             .unwrap(),
//     );
//
//     let account_info = source_account.and_then(|key| get_account_info(key));
//     let source_mint_account = account_info
//         .and_then(|info| Some(info.0.to_string()))
//         .unwrap_or_default(); //get_mint_account(&source_account).unwrap_or_default();
//     let owner_account = account_info
//         .and_then(|info| Some(info.1.to_string()))
//         .unwrap_or_default();
//     let destination = tran.transaction.message.account_keys.get(
//         inst.accounts
//             .get(6)
//             .and_then(|ind| Some(*ind as usize))
//             .unwrap(),
//     );
//     let account_info = destination.and_then(|key| get_account_info(key));
//     let destination_mint_account = account_info
//         .and_then(|info| Some(info.0.to_string()))
//         .unwrap_or_default();
//     SaberSwap {
//         block_slot: block.parent_slot as i64 + 1,
//         parent_slot: block.parent_slot as i64,
//         tx_index: tx_ind as i64,
//         tx_hash,
//         instruction_index: inst_ind as i64,
//         block_time: block.block_time.unwrap_or_default(),
//         owner_account,
//         id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(0)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//
//         authority_base: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(1)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         authority_source: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(2)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         source: source_account
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         base_into: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(4)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         base_from: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(5)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         destination: destination
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         admin_fee_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(7)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         program_id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(8)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         clock_sysvar: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(9)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         amount_in: swap.amount_in as i64,
//         minimum_amount_out: swap.minimum_amount_out as i64,
//         source_mint_account,
//         destination_mint_account,
//     }
// }
// fn create_deposit_entity(
//     block: &ConfirmedBlock,
//     tran: &TransactionWithStatusMeta,
//     tx_ind: usize,
//     inst: &CompiledInstruction,
//     inst_ind: usize,
//     deposit: &DepositData,
// ) -> SaberDeposit {
//     let tx_hash = tran
//         .transaction
//         .signatures
//         .get(0)
//         .and_then(|sig| Some(sig.to_string()))
//         .unwrap_or_default();
//     let token_a = tran.transaction.message.account_keys.get(
//         inst.accounts
//             .get(3)
//             .and_then(|ind| Some(*ind as usize))
//             .unwrap(),
//     );
//     //.and_then(|key| Some(key.to_string()))
//     //.unwrap_or_default();
//     let token_b = tran.transaction.message.account_keys.get(
//         inst.accounts
//             .get(4)
//             .and_then(|ind| Some(*ind as usize))
//             .unwrap(),
//     );
//     //.and_then(|key| Some(key.to_string()))
//     //.unwrap_or_default();
//     let token_a_info = token_a.and_then(|key| get_account_info(key));
//     let token_b_info = token_b.and_then(|key| get_account_info(key));
//     let token_a_mint_account = token_a_info
//         .and_then(|info| Some(info.0.to_string()))
//         .unwrap_or_default();
//     let owner_account = token_a_info
//         .and_then(|info| Some(info.1.to_string()))
//         .unwrap_or_default();
//     let token_b_mint_account = token_b_info
//         .and_then(|info| Some(info.0.to_string()))
//         .unwrap_or_default();
//     // let token_a_mint_account = get_mint_account(&token_a).unwrap_or_default();
//     // let token_b_mint_account = get_mint_account(&token_b).unwrap_or_default();
//     SaberDeposit {
//         block_slot: block.parent_slot as i64 + 1,
//         parent_slot: block.parent_slot as i64,
//         tx_index: tx_ind as i64,
//         tx_hash,
//         instruction_index: inst_ind as i64,
//         block_time: block.block_time.unwrap_or_default(),
//         owner_account,
//         id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(0)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         base_authority: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(1)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         owner_authority: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(2)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_a: token_a
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_b: token_b
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_a_base: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(5)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_b_base: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(6)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         mint_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(7)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(8)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         program_id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(9)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         clock_sysvar: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(10)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_a_amount: deposit.token_a_amount as i64,
//         token_b_amount: deposit.token_b_amount as i64,
//         min_mint_amount: deposit.min_mint_amount as i64,
//         token_a_mint_account,
//         token_b_mint_account,
//     }
// }
// fn create_withdraw_entity(
//     block: &ConfirmedBlock,
//     tran: &TransactionWithStatusMeta,
//     tx_ind: usize,
//     inst: &CompiledInstruction,
//     inst_ind: usize,
//     withdraw: &WithdrawData,
// ) -> SaberWithdraw {
//     let tx_hash = tran
//         .transaction
//         .signatures
//         .get(0)
//         .and_then(|sig| Some(sig.to_string()))
//         .unwrap_or_default();
//     let token_a = tran.transaction.message.account_keys.get(
//         inst.accounts
//             .get(7)
//             .and_then(|ind| Some(*ind as usize))
//             .unwrap(),
//     );
//     let token_b = tran.transaction.message.account_keys.get(
//         inst.accounts
//             .get(8)
//             .and_then(|ind| Some(*ind as usize))
//             .unwrap(),
//     );
//     let token_a_info = token_a.and_then(|key| get_account_info(key));
//     let token_b_info = token_b.and_then(|key| get_account_info(key));
//     let token_a_mint_account = token_a_info
//         .and_then(|info| Some(info.0.to_string()))
//         .unwrap_or_default();
//     let owner_account = token_a_info
//         .and_then(|info| Some(info.1.to_string()))
//         .unwrap_or_default();
//     let token_b_mint_account = token_b_info
//         .and_then(|info| Some(info.0.to_string()))
//         .unwrap_or_default();
//     SaberWithdraw {
//         block_slot: block.parent_slot as i64 + 1,
//         parent_slot: block.parent_slot as i64,
//         tx_index: tx_ind as i64,
//         tx_hash,
//         instruction_index: inst_ind as i64,
//         block_time: block.block_time.unwrap_or_default(),
//         owner_account,
//         id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(0)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         base_authority: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(1)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         owner_authority: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(2)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_mint: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(3)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(4)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_a_swap: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(5)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_b_swap: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(6)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_a_user: token_a
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_b_user: token_b
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         token_a_mint_account,
//         token_b_mint_account,
//         admin_fee_a_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(9)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         admin_fee_b_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(10)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         program_id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(11)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_token_amount: withdraw.pool_token_amount as i64,
//         minimum_token_a_amount: withdraw.minimum_token_a_amount as i64,
//         minimum_token_b_amount: withdraw.minimum_token_b_amount as i64,
//     }
// }
// fn create_withdraw_one_entity(
//     block: &ConfirmedBlock,
//     tran: &TransactionWithStatusMeta,
//     tx_ind: usize,
//     inst: &CompiledInstruction,
//     inst_ind: usize,
//     with_draw_one: &WithdrawOneData,
// ) -> SaberWithdrawOne {
//     let tx_hash = tran
//         .transaction
//         .signatures
//         .get(0)
//         .and_then(|sig| Some(sig.to_string()))
//         .unwrap_or_default();
//     let user_account = tran.transaction.message.account_keys.get(
//         inst.accounts
//             .get(7)
//             .and_then(|ind| Some(*ind as usize))
//             .unwrap(),
//     );
//     let user_account_info = user_account.and_then(|key| get_account_info(key));
//     let owner_account = user_account_info
//         .and_then(|info| Some(info.1.to_string()))
//         .unwrap_or_default();
//     let user_mint_account = user_account_info
//         .and_then(|info| Some(info.0.to_string()))
//         .unwrap_or_default();
//     SaberWithdrawOne {
//         block_slot: block.parent_slot as i64 + 1,
//         parent_slot: block.parent_slot as i64,
//         tx_index: tx_ind as i64,
//         tx_hash,
//         instruction_index: inst_ind as i64,
//         block_time: block.block_time.unwrap_or_default(),
//         owner_account,
//         id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(0)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         swap_authority: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(1)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_authority: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(2)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_mint: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(3)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(4)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         swap_base_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(5)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         swap_quote_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(6)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         user_account: user_account
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         user_mint_account,
//         admin_fee_account: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(8)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         program_id: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(9)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         clock_sysvar: tran
//             .transaction
//             .message
//             .account_keys
//             .get(
//                 inst.accounts
//                     .get(10)
//                     .and_then(|ind| Some(*ind as usize))
//                     .unwrap(),
//             )
//             .and_then(|key| Some(key.to_string()))
//             .unwrap_or_default(),
//         pool_token_amount: with_draw_one.pool_token_amount as i64,
//         minimum_token_amount: with_draw_one.minimum_token_amount as i64,
//     }
// }

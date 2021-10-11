use crate::models::*;
use massbit_chain_solana::data_type::{Pubkey, SolanaBlock, SolanaLogMessages, SolanaTransaction};
use solana_transaction_status::{parse_instruction, ConfirmedBlock, TransactionWithStatusMeta};
use stable_swap_client::instruction::*;
use stable_swap_client::solana_program::program_error::ProgramError;
use uuid::Uuid;

const ADDRESS: &str = "SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ";

fn is_match(pubkeys: &Vec<Pubkey>) -> bool {
    pubkeys.iter().any(|&key| key.to_string() == ADDRESS)
}

pub fn handle_block(block: &SolanaBlock) -> Result<(), Box<dyn std::error::Error>> {
    for tran in &block.block.transactions {
        if !is_match(&tran.transaction.message.account_keys) {
            continue;
        }
        let entities = parse_instructions(&block.block, tran);
    }
    Ok(())
}
fn parse_instructions(block: &ConfirmedBlock, tran: &TransactionWithStatusMeta) {
    println!("{:?}", tran);
    for (ind, inst) in tran.transaction.message.instructions.iter().enumerate() {
        let program_key = inst.program_id(tran.transaction.message.account_keys.as_slice());
        println!("{:?}", program_key);
        let admin_inst = match AdminInstruction::unpack(inst.data.as_slice()) {
            Ok(opt_inst) => opt_inst,
            Err(err) => None,
        };
        if admin_inst.is_none() {
            match SwapInstruction::unpack(inst.data.as_slice()) {
                Ok(opt_inst) => {
                    process_swap_instruction(block, tran, &opt_inst);
                }
                Err(err) => {}
            };
        } else {
            process_admin_instruction(block, tran, admin_inst.as_ref().unwrap());
        }
    }
    let id = Uuid::new_v4().to_simple().to_string();
}

fn process_admin_instruction(
    block: &ConfirmedBlock,
    tran: &TransactionWithStatusMeta,
    instruction: &AdminInstruction,
) {
    match instruction {
        AdminInstruction::RampA(ramp_a) => {}
        AdminInstruction::StopRampA => {}
        AdminInstruction::Pause => {}
        AdminInstruction::Unpause => {}
        AdminInstruction::SetFeeAccount => {}
        AdminInstruction::ApplyNewAdmin => {}
        AdminInstruction::CommitNewAdmin => {}
        AdminInstruction::SetNewFees(fees) => {}
    }
}

fn process_swap_instruction(
    block: &ConfirmedBlock,
    tran: &TransactionWithStatusMeta,
    instruction: &SwapInstruction,
) {
    match instruction {
        SwapInstruction::Initialize(init) => {
            //println!("{:?}", tran.transaction.message.account_keys.as_slice());
            let entity = SaberInit {
                id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(0)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                nonce: init.nonce as i64,
                amp_factor: init.amp_factor as i64,
                admin_trade_fee_numerator: init.fees.admin_trade_fee_numerator as i64,
                admin_trade_fee_denominator: init.fees.admin_trade_fee_denominator as i64,
                admin_withdraw_fee_numerator: init.fees.admin_withdraw_fee_numerator as i64,
                admin_withdraw_fee_denominator: init.fees.admin_withdraw_fee_denominator as i64,
                trade_fee_numerator: init.fees.trade_fee_numerator as i64,
                trade_fee_denominator: init.fees.trade_fee_denominator as i64,
                withdraw_fee_numerator: init.fees.withdraw_fee_numerator as i64,
                withdraw_fee_denominator: init.fees.withdraw_fee_denominator as i64,
                block_height: block.block_height.unwrap_or_default() as i64,
                parent_slot: block.parent_slot as i64,
            };
            entity.save();
        }
        SwapInstruction::Swap(swap) => {
            //println!("{:?}", tran.transaction.message.account_keys.as_slice());
            let swap_entity = SaberSwap {
                id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(0)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                block_height: block.block_height.unwrap_or_default() as i64,
                parent_slot: block.parent_slot as i64,
                authority: tran
                    .transaction
                    .message
                    .account_keys
                    .get(1)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                source: tran
                    .transaction
                    .message
                    .account_keys
                    .get(2)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                base_into: tran
                    .transaction
                    .message
                    .account_keys
                    .get(3)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                base_from: tran
                    .transaction
                    .message
                    .account_keys
                    .get(5)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                destination: tran
                    .transaction
                    .message
                    .account_keys
                    .get(4)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                admin_fee_account: tran
                    .transaction
                    .message
                    .account_keys
                    .get(6)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                program_id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(6)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                clock_sysvar: tran
                    .transaction
                    .message
                    .account_keys
                    .get(8)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                amount_in: swap.amount_in as i64,
                minimum_amount_out: swap.minimum_amount_out as i64,
            };
            swap_entity.save();
        }
        SwapInstruction::Deposit(deposit) => {
            //println!("{:?}", tran.transaction.message.account_keys.as_slice());
            let deposit_entity = SaberDeposit {
                id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(0)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                authority: tran
                    .transaction
                    .message
                    .account_keys
                    .get(1)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_a_authority: tran
                    .transaction
                    .message
                    .account_keys
                    .get(2)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_b_authority: tran
                    .transaction
                    .message
                    .account_keys
                    .get(3)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_a_base: tran
                    .transaction
                    .message
                    .account_keys
                    .get(4)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_b_base: tran
                    .transaction
                    .message
                    .account_keys
                    .get(5)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                mint_account: tran
                    .transaction
                    .message
                    .account_keys
                    .get(6)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                pool_account: tran
                    .transaction
                    .message
                    .account_keys
                    .get(7)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                program_id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(8)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                clock_sysvar: tran
                    .transaction
                    .message
                    .account_keys
                    .get(9)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_a_amount: deposit.token_a_amount as i64,
                token_b_amount: deposit.token_b_amount as i64,
                min_mint_amount: deposit.min_mint_amount as i64,
                block_height: block.block_height.unwrap_or_default() as i64,
                parent_slot: block.parent_slot as i64,
            };
            deposit_entity.save();
        }
        SwapInstruction::Withdraw(withdraw) => {
            //println!("{:?}", tran.transaction.message.account_keys.as_slice());
            let withdraw_entity = SaberWithdraw {
                id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(0)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                authority: tran
                    .transaction
                    .message
                    .account_keys
                    .get(1)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                mint_pool: tran
                    .transaction
                    .message
                    .account_keys
                    .get(2)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                source_pool: tran
                    .transaction
                    .message
                    .account_keys
                    .get(3)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_a_swap: tran
                    .transaction
                    .message
                    .account_keys
                    .get(4)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_b_swap: tran
                    .transaction
                    .message
                    .account_keys
                    .get(5)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_a_user: tran
                    .transaction
                    .message
                    .account_keys
                    .get(6)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                token_b_user: tran
                    .transaction
                    .message
                    .account_keys
                    .get(7)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                admin_fee_a_account: tran
                    .transaction
                    .message
                    .account_keys
                    .get(8)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                admin_fee_b_account: tran
                    .transaction
                    .message
                    .account_keys
                    .get(9)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                program_id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(10)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                pool_token_amount: withdraw.pool_token_amount as i64,
                minimum_token_a_amount: withdraw.minimum_token_a_amount as i64,
                minimum_token_b_amount: withdraw.minimum_token_b_amount as i64,
                block_height: block.block_height.unwrap_or_default() as i64,
                parent_slot: block.parent_slot as i64,
            };
            withdraw_entity.save();
        }
        SwapInstruction::WithdrawOne(with_draw_one) => {
            //println!("{:?}", tran.transaction.message.account_keys.as_slice());
            let with_draw_one_entity = SaberWithdrawOne {
                id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(0)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                authority: tran
                    .transaction
                    .message
                    .account_keys
                    .get(1)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                mint_pool: tran
                    .transaction
                    .message
                    .account_keys
                    .get(2)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                source_pool: tran
                    .transaction
                    .message
                    .account_keys
                    .get(3)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                base_token_from: tran
                    .transaction
                    .message
                    .account_keys
                    .get(4)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                quote_token: tran
                    .transaction
                    .message
                    .account_keys
                    .get(5)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                base_token_to: tran
                    .transaction
                    .message
                    .account_keys
                    .get(6)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                admin_fee_account: tran
                    .transaction
                    .message
                    .account_keys
                    .get(7)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                program_id: tran
                    .transaction
                    .message
                    .account_keys
                    .get(8)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                clock_sysvar: tran
                    .transaction
                    .message
                    .account_keys
                    .get(9)
                    .and_then(|key| Some(key.to_string()))
                    .unwrap_or_default(),
                pool_token_amount: with_draw_one.pool_token_amount as i64,
                minimum_token_amount: with_draw_one.minimum_token_amount as i64,
                block_height: block.block_height.unwrap_or_default() as i64,
                parent_slot: block.parent_slot as i64,
            };
            with_draw_one_entity.save();
        }
    }
}

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
                    process_swap_instruction(block, &opt_inst);
                }
                Err(err) => {}
            };
        } else {
            process_admin_instruction(admin_inst.as_ref().unwrap());
        }
    }
    let id = Uuid::new_v4().to_simple().to_string();
}

fn process_admin_instruction(instruction: &AdminInstruction) {
    match instruction {
        AdminInstruction::RampA(_) => {}
        AdminInstruction::StopRampA => {}
        AdminInstruction::Pause => {}
        AdminInstruction::Unpause => {}
        AdminInstruction::SetFeeAccount => {}
        AdminInstruction::ApplyNewAdmin => {}
        AdminInstruction::CommitNewAdmin => {}
        AdminInstruction::SetNewFees(_) => {}
    }
}

fn process_swap_instruction(block: &ConfirmedBlock, instruction: &SwapInstruction) {
    match instruction {
        SwapInstruction::Initialize(_) => {}
        SwapInstruction::Swap(swap) => {
            let block = SaberSwap {
                id: block.blockhash.clone(),
                amount_in: swap.amount_in as i64,
                minimum_amount_out: swap.minimum_amount_out as i64,
            };
            block.save();
        }
        SwapInstruction::Deposit(_) => {}
        SwapInstruction::Withdraw(_) => {}
        SwapInstruction::WithdrawOne(_) => {}
    }
}

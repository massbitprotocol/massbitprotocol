use massbit::prelude::Arc;
use massbit_chain_solana::data_type::Pubkey;
use solana_client::client_error::ClientError;
use solana_client::rpc_client::RpcClient;
use solana_sdk::account::Account;
use solana_transaction_status::UiPartiallyDecodedInstruction;
use std::str::FromStr;

pub fn parse_partially_decoded_instruction(
    rpc_client: Arc<RpcClient>,
    instruction: &UiPartiallyDecodedInstruction,
) {
    let pubkey = Pubkey::from_str(instruction.program_id.as_str()).unwrap();
    match rpc_client.get_account(&pubkey) {
        Ok(acc) => {
            println!("{:?}", &acc);
            match rpc_client.get_program_accounts(&acc.owner) {
                Ok(vec) => {
                    vec.iter().for_each(|(key, acc)| {
                        println!("{:?}=>{:?}", key, acc);
                    });
                }
                Err(err) => {
                    println!("{:?}", &err);
                }
            }
        }
        Err(err) => {
            println!("{:?}", &err);
        }
    }
    // match rpc_client.get_program_accounts(&pubkey) {
    //     Ok(acc) => {
    //         println!("{:?}", &acc);
    //     }
    //     Err(err) => {
    //         println!("{:?}", &err);
    //     }
    // }
    // match rpc_client.get_token_account(&pubkey) {
    //     Ok(acc) => {
    //         println!("{:?}", &acc);
    //     }
    //     Err(err) => {
    //         println!("{:?}", &err);
    //     }
    //}
}

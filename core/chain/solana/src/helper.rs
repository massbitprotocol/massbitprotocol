//use crate::data_type::Pubkey;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use lazy_static::lazy_static;
use solana_client::client_error::{ClientError, ClientErrorKind, Result as ClientResult};
use solana_client::rpc_client::RpcClient;
use solana_program::account_info::{Account as _, AccountInfo};
use solana_sdk::account::Account;
use spl_token::solana_program::{program_pack::Pack, pubkey::Pubkey};
// use solana_program::pubkey::Pubkey;
// use solana_sdk::program_pack::Pack;
use spl_token::state::Account as TokenAccount;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
lazy_static! {
    static ref SOLANA_CLIENT: Arc<RpcClient> = Arc::new(RpcClient::new(
        env::var("SOLANA_RPC_URL").unwrap_or(String::from("http://194.163.156.242:8899"))
    ));
}

pub fn get_owner_account(account: &Pubkey) -> Option<String> {
    Some(String::from(""))
}

pub fn get_mint_account(account: &Pubkey) -> Option<String> {
    Some(String::from(""))
}

pub fn get_account_info(pubkey: String) -> Option<(String, String, u64)> {
    let res = SOLANA_CLIENT.get_account(
        &solana_program::pubkey::Pubkey::from_str(pubkey.as_str()).unwrap_or_default(),
    );
    res.ok().and_then(|acc| {
        let token_account = TokenAccount::unpack_from_slice(acc.data.as_slice()).ok();
        token_account.map(|token_account| {
            (
                token_account.mint.to_string(),
                token_account.owner.to_string(),
                token_account.amount,
            )
        })
    })
}

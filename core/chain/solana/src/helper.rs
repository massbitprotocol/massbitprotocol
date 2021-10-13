use crate::data_type::Pubkey;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use lazy_static::lazy_static;
use solana_client::client_error::{ClientError, ClientErrorKind, Result as ClientResult};
use solana_client::rpc_client::RpcClient;
use solana_program::account_info::{Account as _, AccountInfo};
use solana_sdk::account::Account;
use std::env;
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

pub fn get_account_info(pubkey: &Pubkey) -> Option<(Pubkey, Pubkey, u64)> {
    let res = SOLANA_CLIENT.get_account(pubkey);
    res.ok()
        .and_then(|acc| unpack_from_slice(acc.data.as_slice()))
}

//https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/state.rs#L86
pub fn unpack_from_slice(src: &[u8]) -> Option<(Pubkey, Pubkey, u64)> {
    let src = array_ref![src, 0, 165];
    let (mint, owner, amount, delegate, state, is_native, delegated_amount, close_authority) =
        array_refs![src, 32, 32, 8, 36, 1, 12, 8, 36];
    let mint = Pubkey::new_from_array(*mint);
    let owner = Pubkey::new_from_array(*owner);
    let amount = u64::from_le_bytes(*amount);
    // let delegate = unpack_coption_key(delegate)?;
    // let state = AccountState::try_from_primitive(state[0])
    // .or(Err(ProgramError::InvalidAccountData))?;
    // let is_native = unpack_coption_u64(is_native)?;
    // let delegated_amount: u64::from_le_bytes(*delegated_amount);
    // let close_authority: unpack_coption_key(close_authority)?;
    Some((mint, owner, amount))
}

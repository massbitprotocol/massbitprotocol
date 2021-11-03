use bytemuck::cast;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

use arrayref::{array_ref, array_refs};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use schemars::{schema_for, JsonSchema};
use std::num::NonZeroU64;

#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
#[repr(u8)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

#[derive(
    Eq,
    PartialEq,
    Copy,
    Clone,
    TryFromPrimitive,
    IntoPrimitive,
    Debug,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
#[repr(u8)]
pub enum OrderType {
    Limit = 0,
    ImmediateOrCancel = 1,
    PostOnly = 2,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(test, proptest(no_params))]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct InitializeMarketInstruction {
    // In the matching engine, all prices and balances are integers.
    // This only works if the smallest representable quantity of the coin
    // is at least a few orders of magnitude larger than the smallest representable
    // quantity of the price currency. The internal representation also relies on
    // on the assumption that every order will have a (quantity x price) value that
    // fits into a u64.
    //
    // If these assumptions are problematic, rejigger the lot sizes.
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub fee_rate_bps: u16,
    pub vault_signer_nonce: u64,
    pub pc_dust_threshold: u64,
}

#[derive(
    PartialEq,
    Eq,
    Copy,
    Clone,
    Debug,
    TryFromPrimitive,
    IntoPrimitive,
    Serialize,
    JsonSchema,
    Deserialize,
)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
#[repr(u8)]
pub enum SelfTradeBehavior {
    DecrementTake = 0,
    CancelProvide = 1,
    AbortTransaction = 2,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, JsonSchema, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SendTakeInstruction {
    pub side: Side,

    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub limit_price: NonZeroU64,

    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub max_coin_qty: NonZeroU64,
    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub max_native_pc_qty_including_fees: NonZeroU64,

    pub min_coin_qty: u64,
    pub min_native_pc_qty: u64,

    pub limit: u16,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, JsonSchema, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct NewOrderInstructionV3 {
    pub side: Side,

    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub limit_price: NonZeroU64,

    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub max_coin_qty: NonZeroU64,
    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub max_native_pc_qty_including_fees: NonZeroU64,

    pub self_trade_behavior: SelfTradeBehavior,

    pub order_type: OrderType,
    pub client_order_id: u64,
    pub limit: u16,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, JsonSchema, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct NewOrderInstructionV2 {
    pub side: Side,
    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub limit_price: NonZeroU64,
    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub max_qty: NonZeroU64,
    pub order_type: OrderType,
    pub client_id: u64,
    pub self_trade_behavior: SelfTradeBehavior,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, JsonSchema, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct NewOrderInstructionV1 {
    pub side: Side,
    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub limit_price: NonZeroU64,
    #[cfg_attr(
        test,
        proptest(strategy = "(1u64..=std::u64::MAX).prop_map(|x| NonZeroU64::new(x).unwrap())")
    )]
    pub max_qty: NonZeroU64,
    pub order_type: OrderType,
    pub client_id: u64,
}
#[derive(PartialEq, Eq, Debug, Clone, JsonSchema, Serialize, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub enum MarketInstruction {
    /// 0. `[writable]` the market to initialize
    /// 1. `[writable]` zeroed out request queue
    /// 2. `[writable]` zeroed out event queue
    /// 3. `[writable]` zeroed out bids
    /// 4. `[writable]` zeroed out asks
    /// 5. `[writable]` spl-token account for the coin currency
    /// 6. `[writable]` spl-token account for the price currency
    /// 7. `[]` coin currency Mint
    /// 8. `[]` price currency Mint
    /// 9. `[]` the rent sysvar
    /// 10. `[]` open orders market authority (optional)
    /// 11. `[]` prune authority (optional, requires open orders market authority)
    /// 12. `[]` crank authority (optional, requires prune authority)
    InitializeMarket(InitializeMarketInstruction),
    /// 0. `[writable]` the market
    /// 1. `[writable]` the OpenOrders account to use
    /// 2. `[writable]` the request queue
    /// 3. `[writable]` the (coin or price currency) account paying for the order
    /// 4. `[signer]` owner of the OpenOrders account
    /// 5. `[writable]` coin vault
    /// 6. `[writable]` pc vault
    /// 7. `[]` spl token program
    /// 8. `[]` the rent sysvar
    /// 9. `[writable]` (optional) the (M)SRM account used for fee discounts
    NewOrder(NewOrderInstructionV1),
    /// 0. `[writable]` market
    /// 1. `[writable]` req_q
    /// 2. `[writable]` event_q
    /// 3. `[writable]` bids
    /// 4. `[writable]` asks
    /// 5. `[writable]` coin fee receivable account
    /// 6. `[writable]` pc fee receivable account
    MatchOrders(u16),
    /// ... `[writable]` OpenOrders
    /// accounts.len() - 4 `[writable]` market
    /// accounts.len() - 3 `[writable]` event queue
    /// accounts.len() - 2 `[writable]` coin fee receivable account
    /// accounts.len() - 1 `[writable]` pc fee receivable account
    ConsumeEvents(u16),
    /// 0. `[]` market
    /// 1. `[writable]` OpenOrders
    /// 2. `[writable]` the request queue
    /// 3. `[signer]` the OpenOrders owner
    CancelOrder(CancelOrderInstruction),
    /// 0. `[writable]` market
    /// 1. `[writable]` OpenOrders
    /// 2. `[signer]` the OpenOrders owner
    /// 3. `[writable]` coin vault
    /// 4. `[writable]` pc vault
    /// 5. `[writable]` coin wallet
    /// 6. `[writable]` pc wallet
    /// 7. `[]` vault signer
    /// 8. `[]` spl token program
    /// 9. `[writable]` (optional) referrer pc wallet
    SettleFunds,
    /// 0. `[]` market
    /// 1. `[writable]` OpenOrders
    /// 2. `[writable]` the request queue
    /// 3. `[signer]` the OpenOrders owner
    CancelOrderByClientId(u64),
    /// 0. `[writable]` market
    /// 1. `[signer]` disable authority
    DisableMarket,
    /// 0. `[writable]` market
    /// 1. `[writable]` pc vault
    /// 2. `[signer]` fee sweeping authority
    /// 3. `[writable]` fee receivable account
    /// 4. `[]` vault signer
    /// 5. `[]` spl token program
    SweepFees,
    /// 0. `[writable]` the market
    /// 1. `[writable]` the OpenOrders account to use
    /// 2. `[writable]` the request queue
    /// 3. `[writable]` the (coin or price currency) account paying for the order
    /// 4. `[signer]` owner of the OpenOrders account
    /// 5. `[writable]` coin vault
    /// 6. `[writable]` pc vault
    /// 7. `[]` spl token program
    /// 8. `[]` the rent sysvar
    /// 9. `[writable]` (optional) the (M)SRM account used for fee discounts
    NewOrderV2(NewOrderInstructionV2),
    /// 0. `[writable]` the market
    /// 1. `[writable]` the OpenOrders account to use
    /// 2. `[writable]` the request queue
    /// 3. `[writable]` the event queue
    /// 4. `[writable]` bids
    /// 5. `[writable]` asks
    /// 6. `[writable]` the (coin or price currency) account paying for the order
    /// 7. `[signer]` owner of the OpenOrders account
    /// 8. `[writable]` coin vault
    /// 9. `[writable]` pc vault
    /// 10. `[]` spl token program
    /// 11. `[]` the rent sysvar
    /// 12. `[writable]` (optional) the (M)SRM account used for fee discounts
    NewOrderV3(NewOrderInstructionV3),
    /// 0. `[writable]` market
    /// 1. `[writable]` bids
    /// 2. `[writable]` asks
    /// 3. `[writable]` OpenOrders
    /// 4. `[signer]` the OpenOrders owner
    /// 5. `[writable]` event_q
    CancelOrderV2(CancelOrderInstructionV2),
    /// 0. `[writable]` market
    /// 1. `[writable]` bids
    /// 2. `[writable]` asks
    /// 3. `[writable]` OpenOrders
    /// 4. `[signer]` the OpenOrders owner
    /// 5. `[writable]` event_q
    CancelOrderByClientIdV2(u64),
    /// 0. `[writable]` market
    /// 1. `[writable]` bids
    /// 2. `[writable]` asks
    /// 3. `[writable]` OpenOrders
    /// 4. `[]`
    SendTake(SendTakeInstruction),
    /// 0. `[writable]` OpenOrders
    /// 1. `[signer]` the OpenOrders owner
    /// 2. `[writable]` the destination account to send rent exemption SOL to
    /// 3. `[]` market
    CloseOpenOrders,
    /// 0. `[writable]` OpenOrders
    /// 1. `[signer]` the OpenOrders owner
    /// 2. `[]` market
    /// 3. `[]` the rent sysvar
    /// 4. `[signer]` open orders market authority (optional).
    InitOpenOrders,
    /// Removes all orders for a given open orders account from the orderbook.
    ///
    /// 0. `[writable]` market
    /// 1. `[writable]` bids
    /// 2. `[writable]` asks
    /// 3. `[signer]` prune authority
    /// 4. `[]` open orders.
    /// 5. `[]` open orders owner.
    /// 6. `[writable]` event queue.
    Prune(u16),
    /// ... `[writable]` OpenOrders
    /// accounts.len() - 3 `[writable]` market
    /// accounts.len() - 2 `[writable]` event queue
    /// accounts.len() - 1 `[signer]` crank authority
    ConsumeEventsPermissioned(u16),
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct CancelOrderInstructionV2 {
    pub side: Side,
    pub order_id: u128,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(test, derive(Arbitrary))]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct CancelOrderInstruction {
    pub side: Side,
    pub order_id: u128,
    pub owner: [u64; 4], // Unused
    pub owner_slot: u8,
}

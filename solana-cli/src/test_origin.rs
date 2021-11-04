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

impl MarketInstruction {
    pub fn pack(&self) -> Vec<u8> {
        bincode::serialize(&(0u8, self)).unwrap()
    }

    pub fn unpack(versioned_bytes: &[u8]) -> Option<Self> {
        if versioned_bytes.len() < 5 || versioned_bytes.len() > 58 {
            return None;
        }
        let (&[version], &discrim, data) = array_refs![versioned_bytes, 1, 4; ..;];
        if version != 0 {
            return None;
        }
        let discrim = u32::from_le_bytes(discrim);
        Some(match (discrim, data.len()) {
            (0, 34) => MarketInstruction::InitializeMarket({
                let data_array = array_ref![data, 0, 34];
                let fields = array_refs![data_array, 8, 8, 2, 8, 8];
                InitializeMarketInstruction {
                    coin_lot_size: u64::from_le_bytes(*fields.0),
                    pc_lot_size: u64::from_le_bytes(*fields.1),
                    fee_rate_bps: u16::from_le_bytes(*fields.2),
                    vault_signer_nonce: u64::from_le_bytes(*fields.3),
                    pc_dust_threshold: u64::from_le_bytes(*fields.4),
                }
            }),
            (1, 32) => MarketInstruction::NewOrder({
                let data_arr = array_ref![data, 0, 32];
                NewOrderInstructionV1::unpack(data_arr)?
            }),
            (2, 2) => {
                let limit = array_ref![data, 0, 2];
                MarketInstruction::MatchOrders(u16::from_le_bytes(*limit))
            }
            (3, 2) => {
                let limit = array_ref![data, 0, 2];
                MarketInstruction::ConsumeEvents(u16::from_le_bytes(*limit))
            }
            (4, 53) => MarketInstruction::CancelOrder({
                let data_array = array_ref![data, 0, 53];
                let fields = array_refs![data_array, 4, 16, 32, 1];
                let side = match u32::from_le_bytes(*fields.0) {
                    0 => Side::Bid,
                    1 => Side::Ask,
                    _ => return None,
                };
                let order_id = u128::from_le_bytes(*fields.1);
                let owner = cast(*fields.2);
                let &[owner_slot] = fields.3;
                CancelOrderInstruction {
                    side,
                    order_id,
                    owner,
                    owner_slot,
                }
            }),
            (5, 0) => MarketInstruction::SettleFunds,
            (6, 8) => {
                let client_id = array_ref![data, 0, 8];
                MarketInstruction::CancelOrderByClientId(u64::from_le_bytes(*client_id))
            }
            (7, 0) => MarketInstruction::DisableMarket,
            (8, 0) => MarketInstruction::SweepFees,
            (9, 36) => MarketInstruction::NewOrderV2({
                let data_arr = array_ref![data, 0, 36];
                let (v1_data_arr, v2_data_arr) = array_refs![data_arr, 32, 4];
                let v1_instr = NewOrderInstructionV1::unpack(v1_data_arr)?;
                let self_trade_behavior = SelfTradeBehavior::try_from_primitive(
                    u32::from_le_bytes(*v2_data_arr).try_into().ok()?,
                )
                .ok()?;
                v1_instr.add_self_trade_behavior(self_trade_behavior)
            }),
            (10, 46) => MarketInstruction::NewOrderV3({
                let data_arr = array_ref![data, 0, 46];
                NewOrderInstructionV3::unpack(data_arr)?
            }),
            (11, 20) => MarketInstruction::CancelOrderV2({
                let data_arr = array_ref![data, 0, 20];
                CancelOrderInstructionV2::unpack(data_arr)?
            }),
            (12, 8) => {
                let client_id = array_ref![data, 0, 8];
                MarketInstruction::CancelOrderByClientIdV2(u64::from_le_bytes(*client_id))
            }
            (13, 46) => MarketInstruction::SendTake({
                let data_arr = array_ref![data, 0, 46];
                SendTakeInstruction::unpack(data_arr)?
            }),
            (14, 0) => MarketInstruction::CloseOpenOrders,
            (15, 0) => MarketInstruction::InitOpenOrders,
            (16, 2) => {
                let limit = array_ref![data, 0, 2];
                MarketInstruction::Prune(u16::from_le_bytes(*limit))
            }
            (17, 2) => {
                let limit = array_ref![data, 0, 2];
                MarketInstruction::ConsumeEventsPermissioned(u16::from_le_bytes(*limit))
            }
            _ => return None,
        })
    }

    #[cfg(test)]
    #[inline]
    pub fn unpack_serde(data: &[u8]) -> Result<Self, ()> {
        match data.split_first() {
            None => Err(()),
            Some((&0u8, rest)) => bincode::deserialize(rest).map_err(|_| ()),
            Some((_, _rest)) => Err(()),
        }
    }
}

impl SendTakeInstruction {
    fn unpack(data: &[u8; 46]) -> Option<Self> {
        let (
            &side_arr,
            &price_arr,
            &max_coin_qty_arr,
            &max_native_pc_qty_arr,
            &min_coin_qty_arr,
            &min_native_pc_qty_arr,
            &limit_arr,
        ) = array_refs![data, 4, 8, 8, 8, 8, 8, 2];

        let side = Side::try_from_primitive(u32::from_le_bytes(side_arr).try_into().ok()?).ok()?;
        let limit_price = NonZeroU64::new(u64::from_le_bytes(price_arr))?;
        let max_coin_qty = NonZeroU64::new(u64::from_le_bytes(max_coin_qty_arr))?;
        let max_native_pc_qty_including_fees =
            NonZeroU64::new(u64::from_le_bytes(max_native_pc_qty_arr))?;
        let min_coin_qty = u64::from_le_bytes(min_coin_qty_arr);
        let min_native_pc_qty = u64::from_le_bytes(min_native_pc_qty_arr);
        let limit = u16::from_le_bytes(limit_arr);

        Some(SendTakeInstruction {
            side,
            limit_price,
            max_coin_qty,
            max_native_pc_qty_including_fees,
            min_coin_qty,
            min_native_pc_qty,
            limit,
        })
    }
}

impl NewOrderInstructionV1 {
    fn unpack(data: &[u8; 32]) -> Option<Self> {
        let (&side_arr, &price_arr, &max_qty_arr, &otype_arr, &client_id_bytes) =
            array_refs![data, 4, 8, 8, 4, 8];
        let client_id = u64::from_le_bytes(client_id_bytes);
        let side = match u32::from_le_bytes(side_arr) {
            0 => Side::Bid,
            1 => Side::Ask,
            _ => return None,
        };
        let limit_price = NonZeroU64::new(u64::from_le_bytes(price_arr))?;
        let max_qty = NonZeroU64::new(u64::from_le_bytes(max_qty_arr))?;
        let order_type = match u32::from_le_bytes(otype_arr) {
            0 => OrderType::Limit,
            1 => OrderType::ImmediateOrCancel,
            2 => OrderType::PostOnly,
            _ => return None,
        };
        Some(NewOrderInstructionV1 {
            side,
            limit_price,
            max_qty,
            order_type,
            client_id,
        })
    }
}

impl NewOrderInstructionV1 {
    pub fn add_self_trade_behavior(
        self,
        self_trade_behavior: SelfTradeBehavior,
    ) -> NewOrderInstructionV2 {
        let NewOrderInstructionV1 {
            side,
            limit_price,
            max_qty,
            order_type,
            client_id,
        } = self;
        NewOrderInstructionV2 {
            side,
            limit_price,
            max_qty,
            order_type,
            client_id,
            self_trade_behavior,
        }
    }
}

impl NewOrderInstructionV3 {
    fn unpack(data: &[u8; 46]) -> Option<Self> {
        let (
            &side_arr,
            &price_arr,
            &max_coin_qty_arr,
            &max_native_pc_qty_arr,
            &self_trade_behavior_arr,
            &otype_arr,
            &client_order_id_bytes,
            &limit_arr,
        ) = array_refs![data, 4, 8, 8, 8, 4, 4, 8, 2];

        let side = Side::try_from_primitive(u32::from_le_bytes(side_arr).try_into().ok()?).ok()?;
        let limit_price = NonZeroU64::new(u64::from_le_bytes(price_arr))?;
        let max_coin_qty = NonZeroU64::new(u64::from_le_bytes(max_coin_qty_arr))?;
        let max_native_pc_qty_including_fees =
            NonZeroU64::new(u64::from_le_bytes(max_native_pc_qty_arr))?;
        let self_trade_behavior = SelfTradeBehavior::try_from_primitive(
            u32::from_le_bytes(self_trade_behavior_arr)
                .try_into()
                .ok()?,
        )
        .ok()?;
        let order_type =
            OrderType::try_from_primitive(u32::from_le_bytes(otype_arr).try_into().ok()?).ok()?;
        let client_order_id = u64::from_le_bytes(client_order_id_bytes);
        let limit = u16::from_le_bytes(limit_arr);

        Some(NewOrderInstructionV3 {
            side,
            limit_price,
            max_coin_qty,
            max_native_pc_qty_including_fees,
            self_trade_behavior,
            order_type,
            client_order_id,
            limit,
        })
    }
}

impl CancelOrderInstructionV2 {
    fn unpack(data: &[u8; 20]) -> Option<Self> {
        let (&side_arr, &oid_arr) = array_refs![data, 4, 16];
        let side = Side::try_from_primitive(u32::from_le_bytes(side_arr).try_into().ok()?).ok()?;
        let order_id = u128::from_le_bytes(oid_arr);
        Some(CancelOrderInstructionV2 { side, order_id })
    }
}

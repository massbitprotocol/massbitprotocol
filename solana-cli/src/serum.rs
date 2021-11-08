#[derive(Clone, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct InitializeMarketInstruction {
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub fee_rate_bps: u16,
    pub vault_signer_nonce: u64,
    pub pc_dust_threshold: u64,
}
impl InitializeMarketInstruction {
    pub fn unpack(input: &[u8]) -> Option<Self> {
        let (&coin_lot_size, &pc_lot_size, &fee_rate_bps, &vault_signer_nonce, &pc_dust_threshold) =
            array_refs![input, 8u32, 8u32, 2u32, 8u32, 8u32];
        InitializeMarketInstruction {
            coin_lot_size: u64::from_le_bytes(coin_lot_size),
            pc_lot_size: u64::from_le_bytes(pc_lot_size),
            fee_rate_bps: u16::from_le_bytes(fee_rate_bps),
            vault_signer_nonce: u64::from_le_bytes(vault_signer_nonce),
            pc_dust_threshold: u64::from_le_bytes(pc_dust_threshold),
        }
    }
}
#[derive(Clone, PartialEq, Debug, Default, Deserialize, Serialize)]
pub struct NewOrderInstructionV1 {
    pub side: Side,
    pub limit_price: NonZeroU64,
    pub max_qty: NonZeroU64,
    pub order_type: OrderType,
    pub client_id: u64,
}
impl NewOrderInstructionV1 {
    pub fn unpack(input: &[u8]) -> Option<Self> {
        let (&side, &limit_price, &max_qty, &order_type, &client_id) =
            array_refs![input, 1u32, 8u32, 8u32, 4u32, 8u32];
        NewOrderInstructionV1 {
            side: Side::unpack(side),
            limit_price: NonZeroU64::unpack(limit_price),
            max_qty: NonZeroU64::unpack(max_qty),
            order_type: OrderType::unpack(order_type),
            client_id: u64::from_le_bytes(client_id),
        }
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    PostOnly,
}
impl OrderType {
    pub fn unpack(input: &[u8]) -> Option<Self> {
        let (&tag_slice, data) = array_refs ! [input , 1u8 ; .. ;];
        let tag_val = u8::from_le_bytes(tag_slice);
        Some(match tag_val {
            0i32 => OrderType::Limit,
            1i32 => OrderType::ImmediateOrCancel,
            2i32 => OrderType::PostOnly,
        })
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask,
}
impl Side {
    pub fn unpack(input: &[u8]) -> Option<Self> {
        let (&tag_slice, data) = array_refs ! [input , 1u8 ; .. ;];
        let tag_val = u8::from_le_bytes(tag_slice);
        Some(match tag_val {
            0i32 => Side::Bid,
            1i32 => Side::Ask,
        })
    }
}
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MarketInstruction {
    InitializeMarket(InitializeMarketInstruction),
    NewOrder(NewOrderInstructionV1),
}
impl MarketInstruction {
    pub fn unpack(input: &[u8]) -> Option<Self> {
        let (&[offset], &tag_slice, data) = array_refs ! [input , 1u16 , 4u8 ; .. ;];
        let tag_val = u32::from_le_bytes(tag_slice);
        Some(match tag_val {
            0i32 => MarketInstruction::InitializeMarket(InitializeMarketInstruction::unpack()),
            1i32 => MarketInstruction::NewOrder(NewOrderInstructionV1::unpack()),
        })
    }
}

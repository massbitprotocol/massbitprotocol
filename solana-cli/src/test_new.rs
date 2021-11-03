#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CancelOrderInstruction {
    pub order_id: i64,
    pub owner: Vec<i64>,
    pub owner_slot: i64,
    pub side: Side,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CancelOrderInstructionV2 {
    pub order_id: i64,
    pub side: Side,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct InitializeMarketInstruction {
    pub coin_lot_size: i64,
    pub fee_rate_bps: i64,
    pub pc_dust_threshold: i64,
    pub pc_lot_size: i64,
    pub vault_signer_nonce: i64,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct NewOrderInstructionV1 {
    pub client_id: i64,
    pub limit_price: i64,
    pub max_qty: i64,
    pub order_type: OrderType,
    pub side: Side,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct NewOrderInstructionV2 {
    pub client_id: i64,
    pub limit_price: i64,
    pub max_qty: i64,
    pub order_type: OrderType,
    pub self_trade_behavior: SelfTradeBehavior,
    pub side: Side,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct NewOrderInstructionV3 {
    pub client_order_id: i64,
    pub limit: i64,
    pub limit_price: i64,
    pub max_coin_qty: i64,
    pub max_native_pc_qty_including_fees: i64,
    pub order_type: OrderType,
    pub self_trade_behavior: SelfTradeBehavior,
    pub side: Side,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    PostOnly,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum SelfTradeBehavior {
    DecrementTake,
    CancelProvide,
    AbortTransaction,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct SendTakeInstruction {
    pub limit: i64,
    pub limit_price: i64,
    pub max_coin_qty: i64,
    pub max_native_pc_qty_including_fees: i64,
    pub min_coin_qty: i64,
    pub min_native_pc_qty: i64,
    pub side: Side,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum Side {
    Bid,
    Ask,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub enum MarketInstructionSideVariant0 {
    SettleFunds,
    DisableMarket,
    SweepFees,
    CloseOpenOrders,
    InitOpenOrders,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant1 {
    #[serde(rename = "InitializeMarket")]
    pub initialize_market: InitializeMarketInstruction,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant2 {
    #[serde(rename = "NewOrder")]
    pub new_order: NewOrderInstructionV1,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant3 {
    #[serde(rename = "MatchOrders")]
    pub match_orders: i64,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant4 {
    #[serde(rename = "ConsumeEvents")]
    pub consume_events: i64,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant5 {
    #[serde(rename = "CancelOrder")]
    pub cancel_order: CancelOrderInstruction,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant6 {
    #[serde(rename = "CancelOrderByClientId")]
    pub cancel_order_by_client_id: i64,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant7 {
    #[serde(rename = "NewOrderV2")]
    pub new_order_v2: NewOrderInstructionV2,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant8 {
    #[serde(rename = "NewOrderV3")]
    pub new_order_v3: NewOrderInstructionV3,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant9 {
    #[serde(rename = "CancelOrderV2")]
    pub cancel_order_v2: CancelOrderInstructionV2,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant10 {
    #[serde(rename = "CancelOrderByClientIdV2")]
    pub cancel_order_by_client_id_v2: i64,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant11 {
    #[serde(rename = "SendTake")]
    pub send_take: SendTakeInstruction,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant12 {
    #[serde(rename = "Prune")]
    pub prune: i64,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct MarketInstructionSideVariant13 {
    #[serde(rename = "ConsumeEventsPermissioned")]
    pub consume_events_permissioned: i64,
}
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MarketInstructionSide {
    Variant0(MarketInstructionSideVariant0),
    Variant1(MarketInstructionSideVariant1),
    Variant2(MarketInstructionSideVariant2),
    Variant3(MarketInstructionSideVariant3),
    Variant4(MarketInstructionSideVariant4),
    Variant5(MarketInstructionSideVariant5),
    Variant6(MarketInstructionSideVariant6),
    Variant7(MarketInstructionSideVariant7),
    Variant8(MarketInstructionSideVariant8),
    Variant9(MarketInstructionSideVariant9),
    Variant10(MarketInstructionSideVariant10),
    Variant11(MarketInstructionSideVariant11),
    Variant12(MarketInstructionSideVariant12),
    Variant13(MarketInstructionSideVariant13),
}
pub type MarketInstruction = MarketInstructionSide;

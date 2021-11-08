type OrderV3 @entity {
    id: ID!
    side: String,
    limit_price: BigInt,
    max_coin_qty: BigInt,
    max_native_pc_qty_including_fees: BigInt,
    self_trade_behavior: String,
    order_type: String,
    client_order_id: BigInt,
    limit: BigInt,
}

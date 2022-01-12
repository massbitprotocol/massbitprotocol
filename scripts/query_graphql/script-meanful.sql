-- Trade history N2H4-USDC

query Process_Event_N2H4_to_USDC_in_2022_01_11
{
  newOrderV3S(where: {
      timestamp_gt:  1641747600,timestamp_lt:  1641834000, market: "14swEM4G2dn7gXUgnqBcJ4ZSAjBEZgavPfbderkm38D7",
  }) {
    id
    bids
    asks
    timestamp
  }
}



-- Trade history BANA-USDC

query new_order_N2H4_to_USDC_in_2022_01_11
{
  newOrderV3S(where: {
      timestamp_gt:  1641747600,timestamp_lt:  1641834000, market: "2Emb4Niq8j4dPLYeoKecvGms8P2GWXYCiao14DpKu1b9",
  }) {
    id
    bids
    asks
    timestamp
  }
}
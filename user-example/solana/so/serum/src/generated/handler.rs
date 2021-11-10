use solana_program::pubkey::Pubkey;
use solana_sdk::account::Account;
use crate::generated::instruction::{MarketInstruction, InitializeMarketInstruction, NewOrderInstructionV1, NewOrderInstructionV2, NewOrderInstructionV3, OrderType, Side, SelfTradeBehavior};
use crate::generated::instruction::MarketInstruction::NewOrderV3;
use crate::models::OrderV3;
use uuid::Uuid;
pub struct SerumHandler {

}
impl SerumHandler {
    pub fn process(&self, program_id: &Pubkey, accounts: &[Account], input: &[u8]) {
        if let Some(market) = MarketInstruction::unpack(input) {
            match market {
                MarketInstruction::InitializeMarket(arg) => {
                    self.process_initialize_market(program_id, accounts, arg);
                }
                MarketInstruction::NewOrder(arg) => {
                    self.process_new_order_v1(program_id, accounts, arg);
                }
                MarketInstruction::MatchOrders(arg) => {}
                MarketInstruction::ConsumeEvents(arg) => {}
                MarketInstruction::CancelOrder(arg) => {}
                MarketInstruction::SettleFunds => {}
                MarketInstruction::CancelOrderByClientId(arg) => {}
                MarketInstruction::DisableMarket => {}
                MarketInstruction::SweepFees => {}
                MarketInstruction::NewOrderV2(arg) => {
                    self.process_new_order_v2(program_id, accounts, arg);
                }
                MarketInstruction::NewOrderV3(arg) => {
                    self.process_new_order_v3(program_id, accounts, arg);
                }
                MarketInstruction::CancelOrderV2(arg) => {}
                MarketInstruction::CancelOrderByClientIdV2(arg) => {}
                MarketInstruction::SendTake(arg) => {}
                MarketInstruction::CloseOpenOrders => {}
                MarketInstruction::InitOpenOrders => {}
                MarketInstruction::Prune(arg) => {}
                MarketInstruction::ConsumeEventsPermissioned(arg) => {}
            }
        }
    }
    pub fn process_initialize_market(&self, program_id: &Pubkey, accounts: &[Account], arg: InitializeMarketInstruction) -> Result<(), anyhow::Error> {
        println!("{:?}", &arg);
        //Create entity
        //entity.save();
        Ok(())
    }
    pub fn process_new_order_v1(&self, program_id: &Pubkey, accounts: &[Account], arg: NewOrderInstructionV1) -> Result<(), anyhow::Error> {
        println!("{:?}", &arg);
        //Create entity
        //entity.save();
        Ok(())
    }
    pub fn process_new_order_v2(&self, program_id: &Pubkey, accounts: &[Account], arg: NewOrderInstructionV2) -> Result<(), anyhow::Error> {
        println!("{:?}", &arg);
        //Create entity
        //entity.save();
        Ok(())
    }
    pub fn process_new_order_v3(&self, program_id: &Pubkey, accounts: &[Account], arg: NewOrderInstructionV3) -> Result<(), anyhow::Error> {
        //println!("{:?}", accounts);
        //Create entity
        let uuid = Uuid::new_v4().to_simple().to_string();
        let side = match arg.side {
            Side::Bid => String::from("0"),
            Side::Ask => String::from("1")
        };
        let order_type = match arg.order_type {
            OrderType::Limit => String::from("0"),
            OrderType::ImmediateOrCancel => String::from("1"),
            OrderType::PostOnly => String::from("2")
        };
        let self_trade_behavior = match arg.self_trade_behavior {
            SelfTradeBehavior::DecrementTake => String::from("DecrementTake"),
            SelfTradeBehavior::CancelProvide => String::from("CancelProvide"),
            SelfTradeBehavior::AbortTransaction => String::from("AbortTransaction")
        };
        let entity = OrderV3 {
            id: uuid,
            side,
            limit_price: arg.limit_price.get() as i64,
            max_coin_qty: arg.max_coin_qty.get() as i64,
            max_native_pc_qty_including_fees: arg.max_native_pc_qty_including_fees.get() as i64,
            self_trade_behavior,
            order_type,
            client_order_id: arg.client_order_id as i64,
            limit: arg.limit as i64
        };
        entity.save();
        Ok(())
    }
}
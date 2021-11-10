use crate::STORE;
use crate::{Entity, EntityFilter, EntityOrder, EntityRange, Value};
use crate::{EntityValue, FromEntity, FromValueTrait, ToMap, ValueFrom};
pub use massbit_drive::{FromEntity, ToMap};
use std::collections::HashMap;

#[derive(Default, Debug, Clone, FromEntity, ToMap)]
pub struct OrderV3 {
    pub id: String,
    pub side: String,   //0-Bid,1-Ask
    pub limit_price: i64,
    pub max_coin_qty: i64,
    pub max_native_pc_qty_including_fees: i64,
//     pub enum SelfTradeBehavior {
//     DecrementTake = 0,
//     CancelProvide = 1,
//     AbortTransaction = 2,
// }
    pub self_trade_behavior: String,        //
    pub order_type: String,
    pub client_order_id: i64,
    pub limit: i64,
}
impl Into<Entity> for OrderV3 {
    fn into(self) -> Entity {
        let map = OrderV3::to_map(self.clone());
        Entity::from(map)
    }
}
impl OrderV3 {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("OrderV3".to_string(), self.clone().into());
        }
    }
    pub fn get(entity_id: &String) -> Option<OrderV3> {
        unsafe {
            let entity = STORE
                .as_mut()
                .unwrap()
                .get("OrderV3".to_string(), entity_id);
            match entity {
                Some(e) => Some(OrderV3::from_entity(&e)),
                None => None,
            }
        }
    }
    pub fn query(
        filter: Option<EntityFilter>,
        order: EntityOrder,
        range: EntityRange,
    ) -> Vec<OrderV3> {
        unsafe {
            STORE
                .as_ref()
                .unwrap()
                .query("OrderV3".to_string(), filter, order, range)
                .iter()
                .map(|e| OrderV3::from_entity(e))
                .collect::<Vec<OrderV3>>()
        }
    }
}

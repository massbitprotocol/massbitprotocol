use index_store::core::{IndexStore, Store};
use rand::prelude::*;
use uuid::Uuid;
const DATABASE_URL: &str = r#"postgres://graph-node:let-me-in@127.0.0.1/"#;
pub static mut STORE: Option<&mut dyn Store> = None;
use std::{thread, time};
use structmap::{FromMap, ToMap};
use structmap_derive::{FromMap, ToMap};

#[derive(Default, Clone, ToMap)]
pub struct BlockTs {
    pub id: String,
    pub block_number: i64,
}

impl Into<structmap::GenericMap> for BlockTs {
    fn into(self) -> structmap::GenericMap {
        BlockTs::to_genericmap(self.clone())
    }
}

impl BlockTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("block_ts".to_string(), self.clone().into());
        }
    }
}

#[derive(Default, Clone, FromMap, ToMap)]
pub struct TransactionSolanaTs {
    pub id: String,
    pub log_messages: String,
    pub signature: String,
    pub block_id: String,
}

impl Into<structmap::GenericMap> for TransactionSolanaTs {
    fn into(self) -> structmap::GenericMap {
        TransactionSolanaTs::to_genericmap(self.clone())
    }
}

impl TransactionSolanaTs {
    pub fn save(&self) {
        unsafe {
            STORE
                .as_mut()
                .unwrap()
                .save("transaction_solana_ts".to_string(), self.clone().into());
        }
    }
}
#[tokio::main]
async fn main() {
    env_logger::init();
    let mut s = IndexStore::new(DATABASE_URL).await;
    let mut store = Some(&mut s);
    let mut rng = rand::thread_rng();
    let size: u32 = rng.gen_range(1000..10000);
    let mut n = 1;
    for _ in 1..=size {
        let block_uuid = Uuid::new_v4();
        let block_id = block_uuid.to_simple().to_string();
        let number: u16 = rng.gen();

        let transaction_uuid = Uuid::new_v4();
        let transaction_ts = TransactionSolanaTs {
            id: transaction_uuid.to_simple().to_string(),
            log_messages: String::from("Log message"),
            signature: String::from("signature"),
            block_id: block_id.clone(),
        };
        store
            .as_mut()
            .unwrap()
            .save("transaction_solana_ts".to_string(), transaction_ts.into());
        let block_ts = BlockTs {
            id: block_id.clone(),
            block_number: number as i64,
        };
        store
            .as_mut()
            .unwrap()
            .save("block_ts".to_string(), block_ts.into());
        let transaction_uuid = Uuid::new_v4();
        let transaction_ts = TransactionSolanaTs {
            id: transaction_uuid.to_simple().to_string(),
            log_messages: String::from("Log message"),
            signature: String::from("signature"),
            block_id: block_id.clone(),
        };
        store
            .as_mut()
            .unwrap()
            .save("transaction_solana_ts".to_string(), transaction_ts.into());
        let sleep_time = rng.gen_range(0..5);
        let sleep_duration = time::Duration::from_millis(sleep_time);
        //let now = time::Instant::now();

        thread::sleep(sleep_duration);
        n += 1;
    }
    println!("Finished!!!");
}

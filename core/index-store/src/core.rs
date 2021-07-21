use diesel::{PgConnection, RunQueryDsl, Connection};
use structmap::GenericMap;
use std::collections::{HashMap, BTreeMap};
//use structmap::value::Value;
use std::time::{SystemTime, SystemTimeError, Duration, Instant};
use std::fmt::{self, Write};
use structmap::value::Value;

const BATCH_SIZE: usize = 1000;
const PERIOD: u128 = 200;        //Period to insert in ms
pub struct IndexStore {
    pub connection_string: String,
    buffer: HashMap<String, Vec<GenericMap>>,
    last_store: Option<Instant>,
}

pub trait Store: Sync + Send {
    fn save(&mut self, entity_name: String, data: GenericMap);
    fn flush(&mut self);
}

impl IndexStore {
    pub fn new (connection: &str) -> IndexStore {
        IndexStore {
            connection_string : connection.to_string(),
            buffer: HashMap::new(),
            last_store: None
        }
    }
    fn check_flush(&mut self, _entity_name: String) {
        let now = Instant::now();
        let elapsed = match self.last_store {
            None => PERIOD,
            Some(last) => now.duration_since(last).as_millis()
        };
        let option_vec =  self.buffer.get_mut(_entity_name.as_str());
        match option_vec {
            None => {}
            Some(vec) => {
                let size = vec.len();
                if size >= BATCH_SIZE || elapsed >= PERIOD {
                    let start = Instant::now();
                    match create_query(_entity_name, vec) {
                        None => {},
                        Some(query) => {
                            let con = PgConnection::establish(&self.connection_string).expect(&format!("Error connecting to {}", self.connection_string));
                            diesel::sql_query(query).execute(&con);
                            self.last_store = Some(Instant::now());
                            vec.clear();
                            log::info!("Insert {:?} records in: {:?} ms.", size, start.elapsed());
                        }
                    }
                }
            }
        };
    }
}
impl Store for IndexStore {
    fn save(&mut self, _entity_name: String, mut _data: GenericMap) {
        match self.buffer.get_mut(_entity_name.as_str()) {
            None => {
                let mut vec = Vec::new();
                vec.push(_data);
                self.buffer.insert(_entity_name, vec);
            }
            Some(vec) => {
                vec.push(_data);
                self.check_flush(_entity_name);
            }
        }
    }
    fn flush(&mut self) {

    }
}
fn create_query(_entity_name : String, vec : &Vec<GenericMap>) -> Option<String> {
    let mut sql_insert = String::new();
    let mut sql_value = String::new();
    write!(sql_insert, "INSERT INTO {} (", _entity_name);
    let mut sep_val = "";
    let mut ind = 0;
    for _data in vec.iter() {
        if ind == 0 {
            let mut sep_field = "";
            for (k, _) in _data {
                write!(sql_insert, "{}{},",sep_field, k);
                sep_field = ",";
            }
            write!(sql_insert,") VALUES ");
        }
        let mut sep_field = "";
        write!(sql_value,"{} (", sep_val);
        for (_, v) in _data {
            match v.string() {
                Some(r) => {
                    write!(sql_value, "{}'{}'", sep_field, r);
                }
                _ => {}
            }
            match v.i64() {
                Some(r) => {
                    write!(sql_value, "{}{}", sep_field, r);
                }
                _ => {}
            }
            sep_field = ",";
        }
        write!(sql_value, ")");
        sep_val = ",";
        ind = ind + 1;
    };
    write!(sql_insert, " {} ", sql_value.as_str());
    Some(sql_insert)


    // Compiling the attributes for the insert query
    // Example: INSERT INTO BlockTs (block_hash,block_height)


    // Compiling the values for the insert query
    // Example: INSERT INTO BlockTs (block_hash,block_height) VALUES ('0x720c…6c50',610)


}
/*
impl Store for IndexStore {
    fn save(&self, _entity_name: String, mut _data: GenericMap) {
        let mut query = format!("INSERT INTO {} (", _entity_name);

        // Compiling the attributes for the insert query
        // Example: INSERT INTO BlockTs (block_hash,block_height)
        for (k, _) in &_data {
            query = format!("{}{},",query, k)
        }
        query = query[0..query.len() - 1].to_string(); // Remove the final `,`
        query = format!("{})",query); // Close the list of attributes

        // Compiling the values for the insert query
        // Example: INSERT INTO BlockTs (block_hash,block_height) VALUES ('0x720c…6c50',610)
        query = format!("{} VALUES (",query); // Add the first `(` for the list of attributes
        for (k, v) in &_data {
            match v.string() {
                Some(r) => {
                    query = format!("{}'{}',",query, r)
                }
                _ => {}
            }
            match v.i64() {
                Some(r) => {
                    query = format!("{}{},",query, r);
                }
                _ => {}
            }
        }
        query = query[0..query.len() - 1].to_string(); // Remove the final `,`
        query = format!("{})",query); // Close the list of attributes
        println!("{}", query); // Inserting the values into the index table
        let c = PgConnection::establish(&self.connection_string).expect(&format!("Error connecting to {}", self.connection_string));
        diesel::sql_query(query).execute(&c);
    }
}
*/
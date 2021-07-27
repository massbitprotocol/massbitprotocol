use diesel::{PgConnection, RunQueryDsl, Connection};
use structmap::GenericMap;
use std::collections::{HashMap, BTreeMap};
//use structmap::value::Value;
use std::time::{SystemTime, SystemTimeError, Duration, Instant};
use std::fmt::{self, Write};
use structmap::value::Value;
use diesel::result::Error;

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
    fn check_and_flush(&mut self, _entity_name: String) {
        let now = Instant::now();
        let elapsed = match self.last_store {
            None => 0,  //First save
            Some(last) => now.duration_since(last).as_millis()
        };
        match self.buffer.get_mut(_entity_name.as_str()) {
            None => {}
            Some(vec) => {
                let size = vec.len();
                if size >= BATCH_SIZE || (elapsed >= PERIOD && size > 0) {
                    let start = Instant::now();
                    match create_query(_entity_name, vec) {
                        None => {},
                        Some(query) => {
                            let con = PgConnection::establish(&self.connection_string).expect(&format!("Error connecting to {}", self.connection_string));
                            match diesel::sql_query(query.as_str()).execute(&con) {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!("[Index-Store] Error {:?} while insert querey {:?}.", err, query.as_str());
                                }
                            }
                            self.last_store = Some(Instant::now());
                            vec.clear();
                            log::info!("[Index-Store] Insert {:?} records in: {:?} ms.", size, start.elapsed());
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
            //Create buffer for first call
            None => {
                let mut vec = Vec::new();
                vec.push(_data);
                self.buffer.insert(_entity_name, vec);
            },
            //Put data into buffer then perform flush to db if buffer size exceeds BATCH_SIZE
            //or elapsed time from last save exceeds PERIOD
            Some(vec) => {
                vec.push(_data);
                self.check_and_flush(_entity_name);
            }
        }
    }
    fn flush(&mut self) {

    }
}
///
/// Create Query with format
/// INSERT INTO {entity_name} ({field1}, {field2}, {field3}) VALUES
/// ('strval11',numberval12, numberval13),
/// ('strval21',numberval22, numberval23),
///
fn create_query(_entity_name : String, vec : &Vec<GenericMap>) -> Option<String> {
    if vec.len() > 0 {
        let fields : Option<Vec<String>> = match vec.get(0) {
            None => None,
            Some(_data) => {
                Some(_data.iter().map(|(k,_)|{k.to_string()}).collect())
            }
        };
        match fields {
            Some(f) => {
                //Store vector of row's string form ('strval11',numberval12, numberval13)
                let row_values : Vec<String> = vec.iter().map(|_data| {
                    let field_values: Vec<String> = _data.iter().map(|(_,v)| {
                        let mut str_val = String::new();
                        match v.string() {
                            Some(r) =>  { write!(str_val, "'{}'", r); }
                            _ => {}
                        };
                        match v.i64() {
                            Some(r) => { write!(str_val, "{}", r); }
                            _ => {}
                        }
                        str_val
                    }).collect();
                    format!("({})",field_values.join(","))
                }).collect();
                Some(format!("INSERT INTO {} ({}) VALUES {};", _entity_name.as_str(), f.join(","), row_values.join(",")))
            }
            None => None
        }
    } else {
        None
    }
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
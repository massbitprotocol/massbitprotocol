use diesel::{PgConnection, RunQueryDsl, Connection};
use structmap::GenericMap;

pub struct IndexStore {
    pub connection_string: String,
}

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
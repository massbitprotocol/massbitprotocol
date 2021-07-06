use super::schema::blocks;
use diesel::{PgConnection, Connection, RunQueryDsl};

#[derive(Insertable)]
#[table_name = "blocks"]
pub struct NewBlock {
    pub number: i64,
}

impl Store for NewBlock{
    // fn save(self, config: &Config) {
    type ConnectionString = String;
    type TableName = String;

    fn save(self) {
        println!("Writing to database {}", self.number);

        println!("{}", Self::ConnectionString);
        // let connection = PgConnection::establish(&String::from(&config.connection_string)).expect(&format!("Error connecting to {}", &config.connection_string));
        // let _ = diesel::insert_into(blocks::table) // Add random hash for the table
        //     // .values(new_block)
        //     .values(self)
        //     .execute(&connection);
    }

    // fn get_table_name(){
    //
    // }
}


// // Store Config
// pub struct Config {
//     pub connection_string: String,
//     pub table_name: String,
// }

pub trait Store {
    type ConnectionString;
    type TableName;

    // fn save(self, config: &Config);
    fn save(self);
}
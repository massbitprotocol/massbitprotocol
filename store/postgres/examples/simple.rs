use store_postgres::layout;

const THINGS_GQL: &str = r#"
    type _Schema_ @fulltext(
        name: "userSearch"
        language: en
        algorithm: rank
        include: [
            {
                entity: "User",
                fields: [
                    { name: "name"},
                    { name: "email"},
                ]
            }
        ]
    ) @fulltext(
        name: "nullableStringsSearch"
        language: en
        algorithm: rank
        include: [
            {
                entity: "NullableStrings",
                fields: [
                    { name: "name"},
                    { name: "description"},
                    { name: "test"},
                ]
            }
        ]
    )

    type Thing @entity {
        id: ID!
        bigThing: Thing!
    }

    enum Color { yellow, red, BLUE }

    type Scalar @entity {
        id: ID,
        bool: Boolean,
        int: Int,
        bigDecimal: BigDecimal,
        bigDecimalArray: [BigDecimal!]!
        string: String,
        strings: [String!],
        bytes: Bytes,
        byteArray: [Bytes!],
        bigInt: BigInt,
        bigIntArray: [BigInt!]!
        color: Color,
    }

    interface Pet {
        id: ID!,
        name: String!
    }

    type Cat implements Pet @entity {
        id: ID!,
        name: String!
    }

    type Dog implements Pet @entity {
        id: ID!,
        name: String!
    }

    type Ferret implements Pet @entity {
        id: ID!,
        name: String!
    }

    type User @entity {
        id: ID!,
        name: String!,
        bin_name: Bytes!,
        email: String!,
        age: Int!,
        seconds_age: BigInt!,
        weight: BigDecimal!,
        coffee: Boolean!,
        favorite_color: Color,
        drinks: [String!]
    }

    type NullableStrings @entity {
        id: ID!,
        name: String,
        description: String,
        test: String
    }
"#;

pub fn main() {
    println!("postgres schema");
    let namespace = "01";
    let result = layout::gen_ddls(THINGS_GQL, namespace);
    match result {
        Ok(res) => {
            for ddl in &res.0 {
                println!("{}", ddl);
            }
            println!("Table names:");
            for table in &res.1 {
                println!("{},", table);
            }
        },
        Err(_) => println!("Invalid schema")
    }
    /*
    let result = layout::gen_ddl(THINGS_GQL, namespace);
    match result {
        Ok(res) => {
            println!("{}", res);
        },
        Err(_) => println!("Invalid schema")
    }
     */
}
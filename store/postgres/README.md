## Graphql schema parser 
Clone from https://github.com/graphprotocol/graph-node/tree/v0.22.0/store/postgres

Modify file relational.rs to generate ddl as list of seperated queries and list table names
# DDL gen CLI

## Usage
```shell
cargo run -- ddlgen --schema schema.graphql --config project.yaml --ouput migrations 
```

## Input templates
`schema.graphql`
```graphql
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
```
`project.yaml`
```yaml

database:
  catalog: dbcatalog
```
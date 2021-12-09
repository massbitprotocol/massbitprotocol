use crate::slog::{info, log};
use async_trait::async_trait;
use massbit_common::prelude::slog::{o, Logger};
use massbit_common::prelude::{slog, tokio};
use massbit_data::log::logger;
use massbit_data::metrics::{Collector, Counter, Gauge, MetricsRegistry, Opts, PrometheusError};
use massbit_data::prelude::{
    q, s, LoadManager, ObjectOrInterface, Query, QueryExecutionError, QueryResult,
};
use massbit_data::schema::{ApiSchema, Schema};
use massbit_data::store::deployment::DeploymentHash;
use massbit_graphql::execution::{ExecutionContext, Query as PreparedQuery, Resolver};
use massbit_graphql::query::{execute_query, QueryExecutionOptions};
use massbit_graphql::{
    opt,
    runner::GraphQlRunner,
    server::{graphql::GraphQlRunner as GraphQlRunnerTrait, GraphQLServer as GraphQLQueryServer},
};
use std::collections::HashMap;
use std::sync::Arc;

const SCHEMA: &str = r#"type InitializeMarket @entity {
    id: ID!,
	market: String,
	request_queue: String,
	event_queue: String,
	bids: String,
	asks: String,
	coin_currency: String,
	price_currency: String,
	coin_currency_mint: String,
	price_currency_mint: String,
	rent_sysvar: String,
	open_orders_market_authority: String,
	prune_authority: String,
	crank_authority: String,
	coin_lot_size: BigInt,
	pc_lot_size: BigInt,
	fee_rate_bps: BigInt,
	vault_signer_nonce: BigInt,
	pc_dust_threshold: BigInt
}
type NewOrder @entity {
    id: ID!,
	market: String,
	open_orders: String,
	request_queue: String,
	account_paying: String,
	owner_openOrders_account: String,
	coin_vault: String,
	pc_vault: String,
	token_program: String,
	rent_sysvar: String,
	SRM_account: String,
	side: String,
	limit_price: BigInt,
	max_qty: BigInt,
	order_type: String,
	client_id: BigInt
}
type MatchOrders @entity {
    id: ID!,
	market: String,
	request_queue: String,
	event_queue: String,
	bids: String,
	asks: String,
	coin_fee: String,
	pc_fee: String,
	value: BigInt
}
type ConsumeEvents @entity {
    id: ID!,
	value: BigInt
}
type CancelOrder @entity {
    id: ID!,
	market: String,
	open_orders: String,
	request_queue: String,
	open_orders_owner: String,
	side: String,
	order_id: String,
	owner: [BigInt],
	owner_slot: BigInt
}
type SettleFunds @entity {
    id: ID!,
	market: String,
	open_orders: String,
	open_orders_owner: String,
	coin_vault: String,
	pc_vault: String,
	coin_wallet: String,
	pc_wallet: String,
	vault_signer: String,
	token_program: String,
	referrer_pc_wallet: String
}
type CancelOrderByClientId @entity {
    id: ID!,
	market: String,
	open_orders: String,
	request_queue: String,
	open_orders_owner: String,
	value: BigInt
}
type DisableMarket @entity {
    id: ID!,
	market: String,
	disable_authority: String
}
type SweepFees @entity {
    id: ID!,
	market: String,
	pc_vault: String,
	fee_sweeping_authority: String,
	fee_receivable_account: String,
	vault_signer: String,
	token_program: String
}
type NewOrderV2 @entity {
    id: ID!,
	market: String,
	open_orders: String,
	request_queue: String,
	account_paying_for_the_order: String,
	open_orders_owner: String,
	coin_vault: String,
	pc_vault: String,
	token_program: String,
	rent_sysvar: String,
	SRM_account: String,
	side: String,
	limit_price: BigInt,
	max_qty: BigInt,
	order_type: String,
	client_id: BigInt,
	self_trade_behavior: String
}
type NewOrderV3 @entity {
    id: ID!,
	market: String,
	open_orders: String,
	request_queue: String,
	event_queue: String,
	bids: String,
	asks: String,
	account_paying_for_the_order: String,
	open_orders_owner: String,
	coin_vault: String,
	pc_vault: String,
	token_program: String,
	rent_sysvar: String,
	side: String,
	limit_price: BigInt,
	max_coin_qty: BigInt,
	max_native_pc_qty_including_fees: BigInt,
	self_trade_behavior: String,
	order_type: String,
	client_order_id: BigInt,
	limit: BigInt
}
type CancelOrderV2 @entity {
    id: ID!,
	market: String,
	bids: String,
	asks: String,
	open_orders: String,
	open_orders_owner: String,
	event_queue: String,
	side: String,
	order_id: String
}
type CancelOrderByClientIdV2 @entity {
    id: ID!,
	market: String,
	bids: String,
	asks: String,
	open_orders: String,
	value: BigInt
}
type SendTake @entity {
    id: ID!,
	market: String,
	bids: String,
	asks: String,
	open_orders: String,
	side: String,
	limit_price: BigInt,
	max_coin_qty: BigInt,
	max_native_pc_qty_including_fees: BigInt,
	min_coin_qty: BigInt,
	min_native_pc_qty: BigInt,
	limit: BigInt
}
type CloseOpenOrders @entity {
    id: ID!,
	open_orders: String,
	open_orders_owner: String,
	destination_to_send_rent_exemption_sol: String,
	market: String
}
type InitOpenOrders @entity {
    id: ID!,
	open_orders: String,
	open_orders_owner: String,
	market: String,
	rent_sysvar: String,
	open_orders_market_authority: String
}
type Prune @entity {
    id: ID!,
	market: String,
	bids: String,
	asks: String,
	prune_authority: String,
	open_orders: String,
	open_orders_owner: String,
	event_queue: String,
	value: BigInt
}
type ConsumeEventsPermissioned @entity {
    id: ID!,
	value: BigInt
}
"#;

const MOCK_SCHEMA: &str = r#"
             scalar ID
             scalar Int
             scalar String
             scalar Boolean

             directive @language(
               language: String = "English"
             ) on FIELD_DEFINITION

             enum Role {
               USER
               ADMIN
             }

             interface Node {
               id: ID!
             }

             type User implements Node @entity {
               id: ID!
               name: String! @language(language: "English")
               role: Role!
             }

             enum User_orderBy {
               id
               name
             }

             input User_filter {
               name_eq: String = "default name",
               name_not: String,
             }

             type Query @entity {
               allUsers(orderBy: User_orderBy, filter: User_filter): [User!]
               anyUserWithAge(age: Int = 99): User
               User: User
             }"#;

/// Mock resolver used in tests that don't need a resolver.
#[derive(Clone)]
pub struct MockResolver;

#[async_trait]
impl Resolver for MockResolver {
    const CACHEABLE: bool = false;

    fn prefetch(
        &self,
        _: &ExecutionContext<Self>,
        _: &q::SelectionSet,
    ) -> Result<Option<q::Value>, Vec<QueryExecutionError>> {
        Ok(None)
    }

    fn resolve_objects<'a>(
        &self,
        _: Option<q::Value>,
        _field: &q::Field,
        _field_definition: &s::Field,
        _object_type: ObjectOrInterface<'_>,
        _arguments: &HashMap<&str, q::Value>,
    ) -> Result<q::Value, QueryExecutionError> {
        Ok(q::Value::Null)
    }

    fn resolve_object(
        &self,
        __: Option<q::Value>,
        _field: &q::Field,
        _field_definition: &s::Field,
        _object_type: ObjectOrInterface<'_>,
        _arguments: &HashMap<&str, q::Value>,
    ) -> Result<q::Value, QueryExecutionError> {
        Ok(q::Value::Null)
    }

    async fn query_permit(&self) -> tokio::sync::OwnedSemaphorePermit {
        Arc::new(tokio::sync::Semaphore::new(1))
            .acquire_owned()
            .await
            .unwrap()
    }
}
/// Execute an introspection query.
async fn introspection_query(schema: Schema, query: &str) -> QueryResult {
    // Create the query
    let query = Query::new(
        graphql_parser::parse_query(query).unwrap().into_static(),
        None,
    );
    // Execute it
    let logger = Logger::root(slog::Discard, o!());
    let load_manager = Arc::new(LoadManager::new(
        &logger,
        Vec::new(),
        Arc::new(MockMetricsRegistry::new()),
    ));

    let options = QueryExecutionOptions {
        resolver: MockResolver,
        deadline: None,
        max_first: u32::MAX,
        max_skip: u32::MAX,
        load_manager: load_manager,
    };

    let schema = Arc::new(ApiSchema::from_api_schema(schema).unwrap());
    let result = match PreparedQuery::new(&logger, schema, None, query, None, 100) {
        Ok(query) => Ok(Arc::try_unwrap(execute_query(query, None, None, options).await).unwrap()),
        Err(e) => Err(e),
    };
    QueryResult::from(result)
}
const QUERY: &str = r#"
      query IntrospectionQuery {
        __schema {
          queryType { name }
          mutationType { name }
          subscriptionType { name}
          types {
            kind
            name
            description
            fields(includeDeprecated: true) {
              name
              description
              args {
                name
                description
                type {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                      ofType {
                        kind
                        name
                        ofType {
                          kind
                          name
                          ofType {
                            kind
                            name
                            ofType {
                              kind
                              name
                              ofType {
                                kind
                                name
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
                defaultValue
              }
              type {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                      ofType {
                        kind
                        name
                        ofType {
                          kind
                          name
                          ofType {
                            kind
                            name
                            ofType {
                              kind
                              name
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
              isDeprecated
              deprecationReason
            }
            inputFields {
              name
              description
              type {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                      ofType {
                        kind
                        name
                        ofType {
                          kind
                          name
                          ofType {
                            kind
                            name
                            ofType {
                              kind
                              name
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
              defaultValue
            }
            interfaces {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                      ofType {
                        kind
                        name
                        ofType {
                          kind
                          name
                          ofType {
                            kind
                            name
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
            enumValues(includeDeprecated: true) {
              name
              description
              isDeprecated
              deprecationReason
            }
            possibleTypes {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                      ofType {
                        kind
                        name
                        ofType {
                          kind
                          name
                          ofType {
                            kind
                            name
                          }
                        }
                      }
                    }
                  }
                }
              }
           }
          }
          directives {
            name
            description
            locations
            args {
              name
              description
              type {
                kind
                name
                ofType {
                  kind
                  name
                  ofType {
                    kind
                    name
                    ofType {
                      kind
                      name
                      ofType {
                        kind
                        name
                        ofType {
                          kind
                          name
                          ofType {
                            kind
                            name
                            ofType {
                              kind
                              name
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
              defaultValue
            }
          }
        }
      }
    "#;
#[tokio::main]
async fn main() {
    let opt = opt::Opt::from_args();
    let logger = logger(opt.debug);
    info!(&logger, "Start graphql HTTP server!");
    let graphql_runner = Arc::new(GraphQlRunner::new(&logger));
    let mut graphql_server = GraphQLQueryServer::new(
        &logger_factory,
        graphql_metrics_registry,
        graphql_runner.clone(),
        node_id.clone(),
    );
    match Schema::parse(MOCK_SCHEMA, DeploymentHash::new("sgd0").unwrap()) {
        Ok(schema) => {
            let result = introspection_query(schema, QUERY).await;
            println!("{:?}", &result);
            //println!("{:?}", &schema);
        }
        Err(err) => println!("{:?}", &err),
    }
}

pub struct MockMetricsRegistry {}

impl MockMetricsRegistry {
    pub fn new() -> Self {
        Self {}
    }
}

impl Clone for MockMetricsRegistry {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl MetricsRegistry for MockMetricsRegistry {
    fn register(&self, _name: &str, _c: Box<dyn Collector>) {
        // Ignore, we do not register metrics
    }

    fn global_counter(
        &self,
        name: &str,
        help: &str,
        const_labels: HashMap<String, String>,
    ) -> Result<Counter, PrometheusError> {
        let opts = Opts::new(name, help).const_labels(const_labels);
        Counter::with_opts(opts)
    }

    fn global_gauge(
        &self,
        name: &str,
        help: &str,
        const_labels: HashMap<String, String>,
    ) -> Result<Gauge, PrometheusError> {
        let opts = Opts::new(name, help).const_labels(const_labels);
        Gauge::with_opts(opts)
    }

    fn unregister(&self, _: Box<dyn Collector>) {
        return;
    }
}

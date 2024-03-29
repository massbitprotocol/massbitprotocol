mod cache;
/// Implementation of the GraphQL execution algorithm.
mod execution;
mod query;
/// Common trait for field resolvers used in the execution.
pub mod resolver;

use massbit_common::prelude::stable_hash::{crypto::SetHasher, StableHasher};

pub use self::execution::*;
pub use self::query::Query;
pub use self::resolver::Resolver;

type QueryHash = <SetHasher as StableHasher>::Out;

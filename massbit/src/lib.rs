/// Traits and types for all system components.
pub mod components;

/// Common data types used throughout Massbit.
pub mod data;

/// Extension traits for external types.
pub mod ext;

/// Utilities.
pub mod util;

/// `CheapClone` trait.
pub mod cheap_clone;

pub mod ipfs_client;

pub mod blockchain;

pub mod runtime;

/// Wrapper for spawning tasks that abort on panic, which is our default.
mod task_spawn;

pub use task_spawn::{
    block_on, spawn, spawn_allow_panic, spawn_blocking, spawn_blocking_allow_panic, spawn_thread,
};

pub use petgraph;
pub use semver;
pub use stable_hash;

/// A prelude that makes all system component traits and data types available.
///
/// Add the following code to import all traits and data types listed below at once.
///
/// ```
/// use massbit::prelude::*;
/// ```
pub mod prelude {
    pub use super::entity;
    pub use ::anyhow;
    pub use anyhow::{anyhow, Context as _, Error};
    pub use async_trait::async_trait;
    pub use bigdecimal;
    pub use futures::future;
    pub use futures::prelude::*;
    pub use futures::stream;
    pub use futures03;
    pub use futures03::compat::{Future01CompatExt, Sink01CompatExt, Stream01CompatExt};
    pub use futures03::future::{FutureExt as _, TryFutureExt};
    pub use futures03::sink::SinkExt as _;
    pub use futures03::stream::{StreamExt as _, TryStreamExt};
    pub use hex;
    pub use lazy_static::lazy_static;
    pub use log::{debug, error, info, warn};
    pub use reqwest;
    pub use serde;
    pub use serde_derive::{Deserialize, Serialize};
    pub use serde_json;
    pub use serde_yaml;
    pub use std::convert::TryFrom;
    pub use std::fmt::Debug;
    pub use std::iter::FromIterator;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use std::time::Duration;
    pub use thiserror;
    pub use tiny_keccak;
    pub use tokio;
    pub use web3;

    pub type DynTryFuture<'a, Ok = (), Err = Error> =
        Pin<Box<dyn futures03::Future<Output = Result<Ok, Err>> + Send + 'a>>;

    pub use crate::blockchain::BlockPtr;

    pub use crate::components::indexer::{
        BlockState, IndexerInstanceManager, RuntimeHost, RuntimeHostBuilder,
    };
    pub use crate::components::link_resolver::{JsonStreamValue, JsonValueStream, LinkResolver};
    pub use crate::components::store::{
        BlockNumber, EntityCache, EntityKey, EntityModification, StoreError,
    };

    pub use crate::data::indexer::{DeploymentHash, IndexerManifest};
    pub use crate::data::store::scalar::{BigDecimal, BigInt, BigIntSign};
    pub use crate::data::store::{
        Attribute, Entity, ToEntityId, ToEntityKey, TryIntoEntity, Value,
    };

    pub use crate::cheap_clone::CheapClone;
    pub use crate::ext::futures::{
        CancelGuard, CancelHandle, CancelToken, CancelableError, FutureExtension,
        SharedCancelGuard, StreamExtension,
    };
    pub use crate::util::cache_weight::CacheWeight;
    pub use crate::util::futures::{retry, TimeoutError};

    macro_rules! static_graphql {
        ($m:ident, $m2:ident, {$($n:ident,)*}) => {
            pub mod $m {
                use graphql_parser::$m2 as $m;
                pub use $m::*;
                $(
                    pub type $n = $m::$n<'static, String>;
                )*
            }
        };
    }

    // Static graphql mods. These are to be phased out, with a preference
    // toward making graphql generic over text. This helps to ease the
    // transition by providing the old graphql-parse 0.2.x API
    static_graphql!(q, query, {
        Document, Value, OperationDefinition, InlineFragment, TypeCondition,
        FragmentSpread, Field, Selection, SelectionSet, FragmentDefinition,
        Directive, VariableDefinition, Type,
    });
    static_graphql!(s, schema, {
        Field, Directive, InterfaceType, ObjectType, Value, TypeDefinition,
        EnumType, Type, Document, ScalarType, InputValue, DirectiveDefinition,
        UnionType, InputObjectType, EnumValue,
    });
}

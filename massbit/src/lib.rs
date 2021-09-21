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

pub mod blockchain;

pub use petgraph;
pub use stable_hash;

/// A prelude that makes all system component traits and data types available.
///
/// Add the following code to import all traits and data types listed below at once.
///
/// ```
/// use massbit::prelude::*;
/// ```
pub mod prelude {
    pub use ::anyhow;
    pub use anyhow::{anyhow, Context as _, Error};
    pub use async_trait::async_trait;
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

    pub use crate::components::store::BlockNumber;

    pub use crate::util::futures::{retry, TimeoutError};

    pub use crate::cheap_clone::CheapClone;
}

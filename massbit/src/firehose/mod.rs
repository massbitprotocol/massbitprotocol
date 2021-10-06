#[path = "stream.rs"]
mod pbstream;

pub mod endpoints;

pub mod dstream {
    pub use super::pbstream::*;
}

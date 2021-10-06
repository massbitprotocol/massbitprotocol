#[path = "stream.rs"]
mod pbstream;

pub mod endpoints;

pub mod stream {
    pub use super::pbstream::*;
}

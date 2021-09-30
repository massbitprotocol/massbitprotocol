#[path = "dfuse.bstream.v1.rs"]
mod pbbstream;

pub mod endpoints;

pub mod bstream {
    pub use super::pbbstream::*;
}

#[path = "chaindata.rs"]
mod pbdstream;

pub mod dstream {
    pub use super::pbdstream::*;
}

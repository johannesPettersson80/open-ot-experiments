//! Definition-file model and hash binding for the OpenOT experiment.
//!
//! This crate intentionally sits above `open-ot-carriage`: it imports the carriage registry and
//! vector contracts, but the carriage crate has no dependency on definition parsing or hashing.

pub mod hash;
pub mod model;
pub mod resolver;
pub mod schema;

pub use hash::{
    DefinitionHash, canonical_definition_bytes_for_hash, canonical_json_bytes,
    canonical_json_bytes_from_str, canonical_json_string_from_str, compute_content_hash,
};
pub use model::*;
pub use resolver::*;
pub use schema::*;

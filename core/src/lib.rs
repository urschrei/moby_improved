//! Client logic for the MOBY Dublin GBFS 3.0 feed.
//!
//! The crate does no network I/O. The host application fetches the feeds and
//! gives the response bodies to this crate for parsing and queries.

mod error;
mod freshness;
pub mod gbfs;
mod position;
mod query;

pub use error::Error;
pub use freshness::Freshness;
pub use position::Position;
pub use query::Candidate;
pub use query::Filter;
pub use query::candidates;

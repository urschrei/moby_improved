//! UniFFI bindings for moby-core.
//!
//! Times cross the boundary as milliseconds since the Unix epoch.

mod error;
mod feed;
mod records;
mod walker;

pub use error::MobyError;
pub use feed::Feed;
pub use feed::vehicle_status_url;
pub use records::Bike;
pub use records::Coordinate;
pub use records::FeedAge;
pub use records::WalkedBike;
pub use walker::Walker;

uniffi::setup_scaffolding!();

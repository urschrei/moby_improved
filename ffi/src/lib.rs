//! UniFFI bindings for moby-core.
//!
//! Times cross the boundary as milliseconds since the Unix epoch.

mod error;
mod feed;
mod records;
mod schedule;
mod walker;

pub use error::MobyError;
pub use feed::Feed;
pub use feed::distance_m;
pub use feed::vehicle_status_url;
pub use records::Bike;
pub use records::Coordinate;
pub use records::FeedAge;
pub use records::TargetStatus;
pub use records::WalkedBike;
pub use schedule::CommuteWindow;
pub use schedule::schedule_is_active;
pub use schedule::schedule_next_boundary_ms;
pub use walker::Walker;

uniffi::setup_scaffolding!();

//! Client logic for the MOBY Dublin GBFS 3.0 feed.
//!
//! The crate does no network I/O. The host application fetches the feeds and
//! gives the response bodies to this crate for parsing and queries.

mod error;
mod freshness;
pub mod gbfs;
mod parking;
mod position;
mod query;
mod schedule;
mod target;
mod walking;

pub use error::Error;
pub use freshness::Freshness;
pub use parking::Bay;
pub use parking::NearbyBay;
pub use parking::ParkingIndex;
pub use position::Position;
pub use query::Candidate;
pub use query::Filter;
pub use query::candidates;
pub use schedule::CommuteWindow;
pub use schedule::Schedule;
pub use schedule::Weekdays;
pub use target::MOVE_THRESHOLD_M;
pub use target::TargetStatus;
pub use target::target_status;
pub use walking::RankedVehicle;
pub use walking::Route;
pub use walking::WalkCache;
pub use walking::WalkingSearch;

//! UniFFI bindings for moby-core.
//!
//! Times cross the boundary as milliseconds since the Unix epoch.

mod error;
mod feed;
mod forecast;
mod parking;
mod records;
mod schedule;
mod walker;

pub use error::MobyError;
pub use feed::Feed;
pub use feed::distance_m;
pub use feed::geofencing_zones_url;
pub use feed::vehicle_status_url;
pub use forecast::ForecastAim;
pub use forecast::ForecastPause;
pub use forecast::ForecastPlan;
pub use forecast::PauseReason;
pub use forecast::PlaceForecast;
pub use forecast::forecast_plan;
pub use forecast::parse_forecast;
pub use forecast::parse_pause;
pub use parking::Parking;
pub use parking::ParkingBay;
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

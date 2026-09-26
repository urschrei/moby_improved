//! Types for the GBFS 3.0 feeds that MOBY publishes.

mod envelope;
mod manifest;
mod vehicle_status;
mod vehicle_types;

pub use envelope::Envelope;
pub use manifest::FeedName;
pub use manifest::Manifest;
pub use vehicle_status::RentalUris;
pub use vehicle_status::Vehicle;
pub use vehicle_status::VehicleStatus;
pub use vehicle_types::FormFactor;
pub use vehicle_types::Propulsion;
pub use vehicle_types::VehicleType;
pub use vehicle_types::VehicleTypes;

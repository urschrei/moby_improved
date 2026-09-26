use jiff::Timestamp;
use serde::Deserialize;

use super::Envelope;

/// The `vehicle_status` feed: every vehicle that is not in an active rental.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct VehicleStatus {
    /// The vehicles in the feed.
    pub vehicles: Vec<Vehicle>,
}

/// One vehicle in the `vehicle_status` feed.
///
/// The specification requires `lat` and `lon` only for vehicles that are not
/// at a station. They are optional here so that one such vehicle does not
/// make the whole feed fail to parse.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Vehicle {
    /// The identifier of the vehicle. The publisher can rotate it.
    pub vehicle_id: String,
    /// The latitude of the vehicle (WGS 84).
    pub lat: Option<f64>,
    /// The longitude of the vehicle (WGS 84).
    pub lon: Option<f64>,
    /// `true` if a rider has reserved the vehicle.
    pub is_reserved: bool,
    /// `true` if the vehicle cannot be rented.
    pub is_disabled: bool,
    /// The identifier of the vehicle type in the `vehicle_types` feed.
    pub vehicle_type_id: Option<String>,
    /// The distance in metres that the vehicle can travel with its current charge.
    pub current_range_meters: Option<f64>,
    /// The charge level from 0 to 1.
    pub current_fuel_percent: Option<f64>,
    /// The time at which the vehicle last reported its status.
    pub last_reported: Option<Timestamp>,
    /// The identifier of the pricing plan.
    pub pricing_plan_id: Option<String>,
    /// Deep links that open the rental app at this vehicle.
    pub rental_uris: Option<RentalUris>,
}

/// Deep links for a vehicle, per platform.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct RentalUris {
    /// The link for Android.
    pub android: Option<String>,
    /// The link for iOS.
    pub ios: Option<String>,
    /// The link for a web browser.
    pub web: Option<String>,
}

impl VehicleStatus {
    /// Parses the feed from a response body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Parse`] if the body is not a `vehicle_status` feed.
    pub fn from_slice(body: &[u8]) -> Result<Envelope<Self>, crate::Error> {
        Envelope::from_slice("vehicle_status", body)
    }
}

impl Vehicle {
    /// Returns `true` if a rider can rent the vehicle now.
    #[must_use]
    pub fn is_available(&self) -> bool {
        !self.is_reserved && !self.is_disabled
    }

    /// Returns the position as (latitude, longitude), if the feed gives one.
    #[must_use]
    pub fn position(&self) -> Option<(f64, f64)> {
        self.lat.zip(self.lon)
    }
}

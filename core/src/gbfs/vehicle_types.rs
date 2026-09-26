use serde::Deserialize;

use super::Envelope;

/// The `vehicle_types` feed.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct VehicleTypes {
    /// The vehicle types in the system.
    pub vehicle_types: Vec<VehicleType>,
}

/// One vehicle type.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct VehicleType {
    /// The identifier that `vehicle_status` refers to.
    pub vehicle_type_id: String,
    /// The general form of the vehicle.
    pub form_factor: FormFactor,
    /// The primary propulsion of the vehicle.
    pub propulsion_type: Propulsion,
    /// The range in metres of the vehicle with a full charge.
    pub max_range_meters: Option<f64>,
}

/// The general form of a vehicle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FormFactor {
    /// A bicycle.
    Bicycle,
    /// A cargo bicycle.
    CargoBicycle,
    /// A car.
    Car,
    /// A moped.
    Moped,
    /// A standing scooter.
    ScooterStanding,
    /// A seated scooter.
    ScooterSeated,
    /// Any other form.
    Other,
    /// A value that this client does not know.
    #[serde(other)]
    Unknown,
}

/// The primary propulsion of a vehicle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Propulsion {
    /// Human power only.
    Human,
    /// Electric assistance to human power.
    ElectricAssist,
    /// Electric motor only.
    Electric,
    /// Combustion engine.
    Combustion,
    /// Combustion engine with a diesel fuel.
    CombustionDiesel,
    /// Hybrid.
    Hybrid,
    /// Plug-in hybrid.
    PlugInHybrid,
    /// Hydrogen fuel cell.
    HydrogenFuelCell,
    /// A value that this client does not know.
    #[serde(other)]
    Unknown,
}

impl VehicleTypes {
    /// Parses the feed from a response body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Parse`] if the body is not a `vehicle_types` feed.
    pub fn from_slice(body: &[u8]) -> Result<Envelope<Self>, crate::Error> {
        Envelope::from_slice("vehicle_types", body)
    }

    /// Returns the vehicle type with the given identifier.
    #[must_use]
    pub fn get(&self, vehicle_type_id: &str) -> Option<&VehicleType> {
        self.vehicle_types
            .iter()
            .find(|vehicle_type| vehicle_type.vehicle_type_id == vehicle_type_id)
    }
}

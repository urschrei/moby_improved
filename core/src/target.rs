use crate::Position;
use crate::gbfs::Vehicle;

/// The state of the vehicle that the rider is walking to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TargetStatus {
    /// The vehicle is available at the same position.
    Available,
    /// The vehicle is available, but at a different position.
    Moved(Position),
    /// The feed does not list the vehicle, or the vehicle is not available.
    /// Another rider has probably rented it.
    Gone,
}

/// The distance in metres that a vehicle must move before it counts as moved.
pub const MOVE_THRESHOLD_M: f64 = 25.0;

/// Finds the vehicle `vehicle_id` in `vehicles` and compares it with the
/// position at which the rider chose it.
#[must_use]
pub fn target_status(vehicles: &[Vehicle], vehicle_id: &str, chosen_at: Position) -> TargetStatus {
    let Some(vehicle) = vehicles
        .iter()
        .find(|vehicle| vehicle.vehicle_id == vehicle_id)
    else {
        return TargetStatus::Gone;
    };
    match vehicle.position() {
        Some((lat, lon)) if vehicle.is_available() => {
            let position = Position::new(lat, lon);
            if chosen_at.distance_m(position) > MOVE_THRESHOLD_M {
                TargetStatus::Moved(position)
            } else {
                TargetStatus::Available
            }
        }
        Some(_) | None => TargetStatus::Gone,
    }
}

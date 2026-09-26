use crate::Position;
use crate::gbfs::Vehicle;

/// The conditions that a vehicle must satisfy to be a candidate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Filter {
    /// The minimum current range in metres.
    pub min_range_m: f64,
}

/// An available vehicle, with its straight-line distance from an origin.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// The identifier of the vehicle.
    pub vehicle_id: String,
    /// The position of the vehicle.
    pub position: Position,
    /// The current range of the vehicle in metres.
    pub range_m: f64,
    /// The great-circle distance in metres from the origin.
    pub straight_line_m: f64,
    /// The deep link that opens the MOBY app at this vehicle on iOS.
    pub ios_rental_uri: Option<String>,
}

/// Returns the vehicles that satisfy the filter, nearest to the origin first.
///
/// A vehicle is a candidate if it is available, it has a position, and its
/// current range is at least `filter.min_range_m`. Vehicles that do not report
/// a range are not candidates.
#[must_use]
pub fn candidates(vehicles: &[Vehicle], origin: Position, filter: Filter) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = vehicles
        .iter()
        .filter(|vehicle| vehicle.is_available())
        .filter_map(|vehicle| {
            let (lat, lon) = vehicle.position()?;
            let range_m = vehicle.current_range_meters?;
            let position = Position::new(lat, lon);
            (range_m >= filter.min_range_m).then(|| Candidate {
                vehicle_id: vehicle.vehicle_id.clone(),
                position,
                range_m,
                straight_line_m: origin.distance_m(position),
                ios_rental_uri: vehicle
                    .rental_uris
                    .as_ref()
                    .and_then(|uris| uris.ios.clone()),
            })
        })
        .collect();
    candidates.sort_by(|a, b| a.straight_line_m.total_cmp(&b.straight_line_m));
    candidates
}

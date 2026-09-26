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
/// This is a linear scan. [`crate::VehicleIndex`] answers the same query from
/// a spatial index, and takes only as many vehicles as the caller uses.
///
/// A vehicle is a candidate if it is available, it has a position, and its
/// current range is at least `filter.min_range_m`. Vehicles that do not report
/// a range are not candidates.
#[must_use]
pub fn candidates(vehicles: &[Vehicle], origin: Position, filter: Filter) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = vehicles
        .iter()
        .filter_map(Listed::from_vehicle)
        .filter(|listed| filter.accepts(listed.range_m))
        .map(|listed| listed.candidate(origin))
        .collect();
    candidates.sort_by(|a, b| a.straight_line_m.total_cmp(&b.straight_line_m));
    candidates
}

impl Filter {
    /// Returns `true` if a vehicle with `range_m` of range satisfies the
    /// filter.
    #[must_use]
    pub fn accepts(&self, range_m: f64) -> bool {
        let Self { min_range_m } = *self;
        range_m >= min_range_m
    }
}

/// A vehicle that can be a candidate: it is available, has a position, and
/// reports a range.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Listed {
    pub(crate) vehicle_id: String,
    pub(crate) position: Position,
    pub(crate) range_m: f64,
    pub(crate) ios_rental_uri: Option<String>,
}

impl Listed {
    /// Returns the vehicle as a listed vehicle, or `None` if it is not
    /// available, has no position, or does not report a range.
    pub(crate) fn from_vehicle(vehicle: &Vehicle) -> Option<Self> {
        if !vehicle.is_available() {
            return None;
        }
        let (lat, lon) = vehicle.position()?;
        Some(Self {
            vehicle_id: vehicle.vehicle_id.clone(),
            position: Position::new(lat, lon),
            range_m: vehicle.current_range_meters?,
            ios_rental_uri: vehicle
                .rental_uris
                .as_ref()
                .and_then(|uris| uris.ios.clone()),
        })
    }

    /// Returns the vehicle as a candidate, with its distance from `origin`.
    pub(crate) fn candidate(&self, origin: Position) -> Candidate {
        Candidate {
            vehicle_id: self.vehicle_id.clone(),
            position: self.position,
            range_m: self.range_m,
            straight_line_m: origin.distance_m(self.position),
            ios_rental_uri: self.ios_rental_uri.clone(),
        }
    }
}

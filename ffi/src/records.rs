use moby_core::Candidate;
use moby_core::Position;
use moby_core::RankedVehicle;

/// A position in degrees (WGS 84).
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Record)]
pub struct Coordinate {
    /// The latitude in degrees.
    pub lat: f64,
    /// The longitude in degrees.
    pub lon: f64,
}

/// An available bike, with its straight-line distance from the origin.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Bike {
    /// The identifier of the vehicle in the feed.
    pub vehicle_id: String,
    /// The position of the bike.
    pub coordinate: Coordinate,
    /// The current range in metres.
    pub range_m: f64,
    /// The great-circle distance in metres from the origin.
    pub straight_line_m: f64,
    /// The deep link that opens the MOBY app at this bike.
    pub rental_uri: Option<String>,
}

/// A bike with its walking route from the origin.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct WalkedBike {
    /// The bike.
    pub bike: Bike,
    /// The length of the walking route in metres.
    pub walking_m: f64,
    /// The expected walking time in seconds.
    pub walking_s: f64,
    /// `true` if routing failed and the route is an estimate.
    pub is_estimate: bool,
}

/// The age of a feed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct FeedAge {
    /// The time since the publisher last updated the feed, in seconds.
    pub age_s: i64,
    /// `true` if the feed is older than the stale threshold.
    pub is_stale: bool,
}

/// The state of the bike that the rider is walking to.
#[derive(Clone, Copy, Debug, PartialEq, uniffi::Enum)]
pub enum TargetStatus {
    /// The bike is available at the same position.
    Available,
    /// The bike is available at a new position.
    Moved {
        /// The new position.
        coordinate: Coordinate,
    },
    /// The bike is not available. Another rider has probably rented it.
    Gone,
}

impl From<moby_core::TargetStatus> for TargetStatus {
    fn from(status: moby_core::TargetStatus) -> Self {
        match status {
            moby_core::TargetStatus::Available => Self::Available,
            moby_core::TargetStatus::Moved(position) => Self::Moved {
                coordinate: position.into(),
            },
            moby_core::TargetStatus::Gone => Self::Gone,
        }
    }
}

impl From<Coordinate> for Position {
    fn from(coordinate: Coordinate) -> Self {
        Position::new(coordinate.lat, coordinate.lon)
    }
}

impl From<Position> for Coordinate {
    fn from(position: Position) -> Self {
        Self {
            lat: position.lat,
            lon: position.lon,
        }
    }
}

impl From<Candidate> for Bike {
    fn from(candidate: Candidate) -> Self {
        Self {
            vehicle_id: candidate.vehicle_id,
            coordinate: candidate.position.into(),
            range_m: candidate.range_m,
            straight_line_m: candidate.straight_line_m,
            rental_uri: candidate.ios_rental_uri,
        }
    }
}

impl From<Bike> for Candidate {
    fn from(bike: Bike) -> Self {
        Self {
            vehicle_id: bike.vehicle_id,
            position: bike.coordinate.into(),
            range_m: bike.range_m,
            straight_line_m: bike.straight_line_m,
            ios_rental_uri: bike.rental_uri,
        }
    }
}

impl From<RankedVehicle> for WalkedBike {
    fn from(ranked: RankedVehicle) -> Self {
        Self {
            bike: ranked.candidate.into(),
            walking_m: ranked.route.distance_m,
            walking_s: ranked.route.duration_s,
            is_estimate: ranked.is_estimate,
        }
    }
}

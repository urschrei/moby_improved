use geo::Distance as _;
use geo::Haversine;
use geo::Point;

/// A point on the surface of the earth (WGS 84), in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    /// The latitude in degrees.
    pub lat: f64,
    /// The longitude in degrees.
    pub lon: f64,
}

impl Position {
    /// Makes a position from a latitude and a longitude in degrees.
    #[must_use]
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }

    /// Returns the great-circle distance in metres to another position.
    ///
    /// A walking route between the two positions is never shorter than this
    /// distance.
    #[must_use]
    pub fn distance_m(self, other: Self) -> f64 {
        Haversine.distance(Point::from(self), Point::from(other))
    }
}

impl From<Position> for Point {
    fn from(position: Position) -> Self {
        Point::new(position.lon, position.lat)
    }
}

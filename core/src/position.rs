/// A point on the surface of the earth (WGS 84), in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    /// The latitude in degrees.
    pub lat: f64,
    /// The longitude in degrees.
    pub lon: f64,
}

/// The mean radius of the earth in metres (IUGG).
const EARTH_RADIUS_M: f64 = 6_371_008.8;

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
        let (lat_a, lat_b) = (self.lat.to_radians(), other.lat.to_radians());
        let half_dlat = (lat_b - lat_a) / 2.0;
        let half_dlon = (other.lon - self.lon).to_radians() / 2.0;
        let h = half_dlat.sin().powi(2) + lat_a.cos() * lat_b.cos() * half_dlon.sin().powi(2);
        2.0 * EARTH_RADIUS_M * h.sqrt().min(1.0).asin()
    }
}

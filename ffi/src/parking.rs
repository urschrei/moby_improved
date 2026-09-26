use std::sync::Arc;

use moby_core::ParkingIndex;
use moby_core::gbfs::GeofencingZones;

use crate::Coordinate;
use crate::MobyError;

/// The parking bays for one vehicle type, from a `geofencing_zones` feed.
#[derive(Debug, uniffi::Object)]
pub struct Parking {
    index: ParkingIndex,
}

/// A parking bay near a query position.
#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct ParkingBay {
    /// The centroid of the bay.
    pub centroid: Coordinate,
    /// The outline of the bay.
    pub outline: Vec<Coordinate>,
    /// The great-circle distance in metres from the query position. It is
    /// zero inside the bay.
    pub distance_m: f64,
}

#[uniffi::export]
impl Parking {
    /// Parses a `geofencing_zones` response body and indexes the bays in
    /// which a vehicle of type `vehicle_type_id` can end a ride.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is not a `geofencing_zones` feed.
    #[uniffi::constructor]
    pub fn parse(body: &[u8], vehicle_type_id: &str) -> Result<Arc<Self>, MobyError> {
        let zones = GeofencingZones::from_slice(body)?;
        Ok(Arc::new(Self {
            index: ParkingIndex::new(&zones.data, vehicle_type_id),
        }))
    }

    /// Returns the number of bays.
    #[must_use]
    pub fn bay_count(&self) -> u32 {
        u32::try_from(self.index.len()).unwrap_or(u32::MAX)
    }

    /// Returns the `k` bays nearest to `coordinate`, nearest first.
    #[must_use]
    pub fn nearest(&self, coordinate: Coordinate, k: u32) -> Vec<ParkingBay> {
        self.index
            .nearest(coordinate.into(), k as usize)
            .into_iter()
            .map(|nearby| ParkingBay {
                centroid: nearby.bay.centroid.into(),
                outline: nearby.bay.outline.into_iter().map(Into::into).collect(),
                distance_m: nearby.distance_m,
            })
            .collect()
    }
}

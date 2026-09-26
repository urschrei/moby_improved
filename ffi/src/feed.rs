use std::sync::Arc;

use jiff::SignedDuration;
use jiff::Timestamp;
use moby_core::Filter;
use moby_core::Freshness;
use moby_core::SharedNearest;
use moby_core::VehicleIndex;
use moby_core::gbfs::Envelope;
use moby_core::gbfs::FeedName;
use moby_core::gbfs::Manifest;
use moby_core::gbfs::VehicleStatus;

use crate::Bike;
use crate::Coordinate;
use crate::FeedAge;
use crate::MobyError;
use crate::TargetStatus;

/// A parsed `vehicle_status` feed.
#[derive(Debug, uniffi::Object)]
pub struct Feed {
    envelope: Envelope<VehicleStatus>,
    index: Arc<VehicleIndex>,
}

/// Returns the URL of the `vehicle_status` feed from a `gbfs.json` body.
///
/// # Errors
///
/// Returns an error if the body does not parse or does not list the feed.
#[uniffi::export]
pub fn vehicle_status_url(manifest: &[u8]) -> Result<String, MobyError> {
    let manifest = Manifest::from_slice(manifest)?;
    Ok(manifest.data.require(FeedName::VehicleStatus)?.to_owned())
}

/// Returns the URL of the `geofencing_zones` feed from a `gbfs.json` body.
///
/// # Errors
///
/// Returns an error if the body does not parse or does not list the feed.
#[uniffi::export]
pub fn geofencing_zones_url(manifest: &[u8]) -> Result<String, MobyError> {
    let manifest = Manifest::from_slice(manifest)?;
    Ok(manifest.data.require(FeedName::GeofencingZones)?.to_owned())
}

/// Returns the great-circle distance in metres between two positions.
#[uniffi::export]
#[must_use]
pub fn distance_m(a: Coordinate, b: Coordinate) -> f64 {
    moby_core::Position::from(a).distance_m(b.into())
}

#[uniffi::export]
impl Feed {
    /// Parses a `vehicle_status` response body.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is not a `vehicle_status` feed.
    #[uniffi::constructor]
    pub fn parse(body: &[u8]) -> Result<Arc<Self>, MobyError> {
        let envelope = VehicleStatus::from_slice(body)?;
        let index = Arc::new(VehicleIndex::new(&envelope.data.vehicles));
        Ok(Arc::new(Self { envelope, index }))
    }

    /// Returns the time of the last update, in milliseconds since the epoch.
    #[must_use]
    pub fn last_updated_ms(&self) -> i64 {
        self.envelope.last_updated.as_millisecond()
    }

    /// Returns the number of vehicles in the feed.
    #[must_use]
    pub fn vehicle_count(&self) -> u32 {
        u32::try_from(self.envelope.data.vehicles.len()).unwrap_or(u32::MAX)
    }

    /// Returns the age of the feed at `now_ms`.
    #[must_use]
    pub fn age(&self, now_ms: i64, stale_after_s: i64) -> FeedAge {
        let now = Timestamp::from_millisecond(now_ms).unwrap_or(Timestamp::MAX);
        let freshness = Freshness::measure(
            self.envelope.last_updated,
            now,
            SignedDuration::from_secs(stale_after_s),
        );
        FeedAge {
            age_s: freshness.age.as_secs(),
            is_stale: freshness.is_stale,
        }
    }

    /// Returns the available bikes with at least `min_range_m` of range,
    /// nearest to `origin` first.
    #[must_use]
    pub fn bikes(&self, origin: Coordinate, min_range_m: f64) -> Vec<Bike> {
        self.index
            .nearest(origin.into(), Filter { min_range_m })
            .map(Bike::from)
            .collect()
    }
}

impl Feed {
    /// Returns the available bikes with at least `min_range_m` of range,
    /// nearest to `origin` first, as an iterator that shares the index.
    pub(crate) fn nearest(&self, origin: Coordinate, min_range_m: f64) -> SharedNearest {
        self.index
            .shared_nearest(origin.into(), Filter { min_range_m })
    }
}

#[uniffi::export]
impl Feed {
    /// Returns the state of the bike `vehicle_id`, which the rider chose at
    /// `chosen_at`.
    #[must_use]
    pub fn target_status(&self, vehicle_id: &str, chosen_at: Coordinate) -> TargetStatus {
        moby_core::target_status(&self.envelope.data.vehicles, vehicle_id, chosen_at.into()).into()
    }
}

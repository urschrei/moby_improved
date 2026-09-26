use std::sync::Arc;

use jiff::SignedDuration;
use jiff::Timestamp;
use moby_core::Filter;
use moby_core::Freshness;
use moby_core::gbfs::Envelope;
use moby_core::gbfs::FeedName;
use moby_core::gbfs::Manifest;
use moby_core::gbfs::VehicleStatus;

use crate::Bike;
use crate::Coordinate;
use crate::FeedAge;
use crate::MobyError;

/// A parsed `vehicle_status` feed.
#[derive(Debug, uniffi::Object)]
pub struct Feed {
    envelope: Envelope<VehicleStatus>,
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

#[uniffi::export]
impl Feed {
    /// Parses a `vehicle_status` response body.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is not a `vehicle_status` feed.
    #[uniffi::constructor]
    pub fn parse(body: &[u8]) -> Result<Arc<Self>, MobyError> {
        Ok(Arc::new(Self {
            envelope: VehicleStatus::from_slice(body)?,
        }))
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
        moby_core::candidates(
            &self.envelope.data.vehicles,
            origin.into(),
            Filter { min_range_m },
        )
        .into_iter()
        .map(Bike::from)
        .collect()
    }
}

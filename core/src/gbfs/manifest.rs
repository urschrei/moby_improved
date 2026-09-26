use serde::Deserialize;

use super::Envelope;

/// The `gbfs.json` auto-discovery feed, which lists the URL of each feed.
#[derive(Clone, Debug, PartialEq)]
pub struct Manifest {
    feeds: Vec<FeedEntry>,
}

/// The name of a feed in the manifest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeedName {
    /// `gbfs`: the manifest itself.
    Gbfs,
    /// `gbfs_versions`
    GbfsVersions,
    /// `system_information`
    SystemInformation,
    /// `vehicle_types`
    VehicleTypes,
    /// `station_information`
    StationInformation,
    /// `station_status`
    StationStatus,
    /// `vehicle_status`
    VehicleStatus,
    /// `system_alerts`
    SystemAlerts,
    /// `system_regions`
    SystemRegions,
    /// `system_pricing_plans`
    SystemPricingPlans,
    /// `geofencing_zones`
    GeofencingZones,
    /// A feed that this client does not know.
    #[serde(other)]
    Unknown,
}

impl Manifest {
    /// Parses the manifest from a response body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Parse`] if the body is not a GBFS 3.0 manifest.
    pub fn from_slice(body: &[u8]) -> Result<Envelope<Self>, crate::Error> {
        let envelope: Envelope<ManifestData> = Envelope::from_slice("gbfs", body)?;
        Ok(Envelope {
            last_updated: envelope.last_updated,
            ttl: envelope.ttl,
            version: envelope.version,
            data: Self {
                feeds: envelope.data.feeds,
            },
        })
    }

    /// Returns the URL of a feed, or `None` if the manifest does not list it.
    #[must_use]
    pub fn url(&self, name: FeedName) -> Option<&str> {
        self.feeds
            .iter()
            .find(|feed| feed.name == name)
            .map(|feed| feed.url.as_str())
    }

    /// Returns the URL of a feed that the client needs.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::MissingFeed`] if the manifest does not list it.
    pub fn require(&self, name: FeedName) -> Result<&str, crate::Error> {
        self.url(name)
            .ok_or(crate::Error::MissingFeed(name.as_str()))
    }
}

impl FeedName {
    /// Returns the feed name as the GBFS specification spells it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gbfs => "gbfs",
            Self::GbfsVersions => "gbfs_versions",
            Self::SystemInformation => "system_information",
            Self::VehicleTypes => "vehicle_types",
            Self::StationInformation => "station_information",
            Self::StationStatus => "station_status",
            Self::VehicleStatus => "vehicle_status",
            Self::SystemAlerts => "system_alerts",
            Self::SystemRegions => "system_regions",
            Self::SystemPricingPlans => "system_pricing_plans",
            Self::GeofencingZones => "geofencing_zones",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Deserialize)]
struct ManifestData {
    feeds: Vec<FeedEntry>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct FeedEntry {
    name: FeedName,
    url: String,
}

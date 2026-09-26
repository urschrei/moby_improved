use serde::Deserialize;

use super::Envelope;

/// The `geofencing_zones` feed: zones with rules for riding and parking.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GeofencingZones {
    /// The zones, as a GeoJSON feature collection.
    pub geofencing_zones: ZoneCollection,
    /// The rules for places outside every zone.
    #[serde(default)]
    pub global_rules: Vec<Rule>,
}

/// A GeoJSON feature collection of zones.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ZoneCollection {
    /// The zones. If zones overlap, the first zone has precedence.
    pub features: Vec<Zone>,
}

/// One zone: a GeoJSON feature with a multipolygon geometry.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Zone {
    /// The area of the zone.
    pub geometry: ZoneGeometry,
    /// The rules of the zone.
    pub properties: ZoneProperties,
}

/// A GeoJSON multipolygon, with positions as `[lon, lat]`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ZoneGeometry {
    /// The polygons. Each polygon is an exterior ring followed by holes.
    pub coordinates: Vec<Vec<Vec<[f64; 2]>>>,
}

/// The properties of a zone.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ZoneProperties {
    /// The rules of the zone. For a vehicle, the first rule that applies to
    /// its type has precedence.
    #[serde(default)]
    pub rules: Vec<Rule>,
}

/// A rule for some or all vehicle types.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Rule {
    /// The vehicle types to which the rule applies. `None` means all types.
    pub vehicle_type_ids: Option<Vec<String>>,
    /// `true` if a rider can start a ride here.
    pub ride_start_allowed: bool,
    /// `true` if a rider can end a ride here.
    pub ride_end_allowed: bool,
    /// `true` if a rider can ride through.
    pub ride_through_allowed: bool,
}

impl GeofencingZones {
    /// Parses the feed from a response body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Parse`] if the body is not a `geofencing_zones` feed.
    pub fn from_slice(body: &[u8]) -> Result<Envelope<Self>, crate::Error> {
        Envelope::from_slice("geofencing_zones", body)
    }
}

impl Rule {
    /// Returns `true` if the rule applies to the vehicle type.
    #[must_use]
    pub fn applies_to(&self, vehicle_type_id: &str) -> bool {
        self.vehicle_type_ids
            .as_ref()
            .is_none_or(|ids| ids.iter().any(|id| id == vehicle_type_id))
    }
}

impl ZoneProperties {
    /// Returns the rule that applies to the vehicle type, if any.
    #[must_use]
    pub fn rule_for(&self, vehicle_type_id: &str) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|rule| rule.applies_to(vehicle_type_id))
    }
}

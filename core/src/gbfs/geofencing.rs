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
#[serde(try_from = "serde_json::Value")]
pub struct Zone {
    /// The identifier of the zone: see [`zone_hash`].
    pub hash: String,
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

/// Returns the identifier of a zone: the blake3 hash, in hexadecimal, of its
/// GeoJSON feature as `serde_json` writes it (keys in order, no spaces).
///
/// The feed gives zones no identifier. A zone whose geometry or rules change
/// gets a new hash.
#[must_use]
pub fn zone_hash(feature: &serde_json::Value) -> String {
    blake3::hash(feature.to_string().as_bytes())
        .to_hex()
        .to_string()
}

impl TryFrom<serde_json::Value> for Zone {
    type Error = serde_json::Error;

    fn try_from(feature: serde_json::Value) -> Result<Self, Self::Error> {
        let hash = zone_hash(&feature);
        let fields: ZoneFields = serde_json::from_value(feature)?;
        Ok(Self {
            hash,
            geometry: fields.geometry,
            properties: fields.properties,
        })
    }
}

/// The fields of a zone that the crate reads.
#[derive(Deserialize)]
struct ZoneFields {
    geometry: ZoneGeometry,
    properties: ZoneProperties,
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

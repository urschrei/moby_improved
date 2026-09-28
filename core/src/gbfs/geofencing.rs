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
///
/// A zone parses from a `serde_json::Value`, so that it gets its hash, and a
/// parse error in a zone has no line or column.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(try_from = "serde_json::Value")]
pub struct Zone {
    /// The identifier of the zone.
    pub hash: ZoneHash,
    /// The area of the zone.
    pub geometry: ZoneGeometry,
    /// The rules of the zone.
    pub properties: ZoneProperties,
}

/// The identifier of a zone: the blake3 hash, in 64 lowercase hexadecimal
/// digits, of its GeoJSON feature as `serde_json` writes it (keys in order,
/// no spaces).
///
/// The feed gives zones no identifier. A zone whose geometry or rules change
/// gets a new hash. `moby-collector` computes the same hash.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(try_from = "String")]
pub struct ZoneHash(String);

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

impl ZoneHash {
    /// Returns the hash of a GeoJSON feature.
    ///
    /// The text of a number comes from the parsed value, so the crate
    /// parses floats with the `float_roundtrip` feature of `serde_json`.
    /// The `preserve_order` and `arbitrary_precision` features would change
    /// every hash.
    #[must_use]
    pub fn of(feature: &serde_json::Value) -> Self {
        Self(
            blake3::hash(feature.to_string().as_bytes())
                .to_hex()
                .to_string(),
        )
    }

    /// Returns the hash in hexadecimal.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<serde_json::Value> for Zone {
    type Error = serde_json::Error;

    fn try_from(feature: serde_json::Value) -> Result<Self, Self::Error> {
        let hash = ZoneHash::of(&feature);
        let fields: ZoneFields = serde_json::from_value(feature)?;
        Ok(Self {
            hash,
            geometry: fields.geometry,
            properties: fields.properties,
        })
    }
}

impl TryFrom<String> for ZoneHash {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let valid = text.len() == 64
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if valid {
            Ok(Self(text))
        } else {
            Err(format!("not a zone hash: {text:?}"))
        }
    }
}

impl std::fmt::Display for ZoneHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ZoneHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for ZoneHash {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// The fields of a zone that the crate reads.
#[derive(Deserialize)]
struct ZoneFields {
    geometry: ZoneGeometry,
    properties: ZoneProperties,
}

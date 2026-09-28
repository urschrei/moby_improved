//! Tests against feed responses captured from the MOBY Dublin GBFS feed.

use moby_core::gbfs::FeedName;
use moby_core::gbfs::FormFactor;
use moby_core::gbfs::GeofencingZones;
use moby_core::gbfs::Manifest;
use moby_core::gbfs::Propulsion;
use moby_core::gbfs::VehicleStatus;
use moby_core::gbfs::VehicleTypes;
use moby_core::gbfs::ZoneHash;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

fn schema_errors(feed: &str) -> Vec<String> {
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(format!(
            "{}/tests/schemas/{feed}.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    let instance: serde_json::Value = serde_json::from_slice(&fixture(feed)).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    validator
        .iter_errors(&instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

#[test]
fn fixtures_conform_to_the_gbfs_3_0_schemas() {
    for feed in [
        "gbfs",
        "vehicle_types",
        "vehicle_status",
        "station_information",
        "station_status",
        "geofencing_zones",
    ] {
        assert_eq!(schema_errors(feed), Vec::<String>::new(), "{feed}");
    }
}

#[test]
fn system_information_has_one_known_deviation() {
    let errors = schema_errors("system_information");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("/data/feed_contact_email"),
        "{errors:?}"
    );
}

#[test]
fn manifest_gives_the_url_of_each_feed() {
    let manifest = Manifest::from_slice(&fixture("gbfs")).unwrap();

    assert_eq!(manifest.version, "3.0");
    assert_eq!(
        manifest.data.url(FeedName::VehicleStatus),
        Some("https://moby-move.rideatom.com/gbfs/v3_0/en/vehicle_status?id=2023")
    );
    assert_eq!(manifest.data.url(FeedName::SystemAlerts), None);
    assert!(matches!(
        manifest.data.require(FeedName::SystemRegions),
        Err(moby_core::Error::MissingFeed("system_regions"))
    ));
}

#[test]
fn manifest_ignores_feeds_it_does_not_know() {
    let body = br#"{"last_updated":"2026-09-26T11:17:28.426Z","ttl":0,"version":"3.0",
        "data":{"feeds":[{"name":"future_feed","url":"https://example.com/f"},
                         {"name":"vehicle_status","url":"https://example.com/v"}]}}"#;
    let manifest = Manifest::from_slice(body).unwrap();

    assert_eq!(
        manifest.data.url(FeedName::VehicleStatus),
        Some("https://example.com/v")
    );
}

#[test]
fn vehicle_status_parses_every_vehicle() {
    let feed = VehicleStatus::from_slice(&fixture("vehicle_status")).unwrap();
    let vehicles = &feed.data.vehicles;

    assert!(vehicles.len() > 500, "{}", vehicles.len());
    assert!(vehicles.iter().all(|vehicle| vehicle.position().is_some()));
    assert!(vehicles.iter().all(|vehicle| {
        vehicle
            .rental_uris
            .as_ref()
            .and_then(|uris| uris.ios.as_ref())
            .is_some()
    }));
}

#[test]
fn vehicle_without_range_parses() {
    let body = br#"{"last_updated":"2026-09-26T11:17:28Z","ttl":0,"version":"3.0",
        "data":{"vehicles":[{"vehicle_id":"1","lat":53.3,"lon":-6.2,
        "is_reserved":false,"is_disabled":true,"vehicle_type_id":"3646"}]}}"#;
    let feed = VehicleStatus::from_slice(body).unwrap();
    let vehicle = &feed.data.vehicles[0];

    assert_eq!(vehicle.current_range_meters, None);
    assert_eq!(vehicle.rental_uris, None);
    assert!(!vehicle.is_available());
}

#[test]
fn malformed_body_names_the_feed() {
    let error = VehicleStatus::from_slice(b"<html>").unwrap_err();

    assert!(
        error
            .to_string()
            .starts_with("cannot parse vehicle_status feed"),
        "{error}"
    );
}

#[test]
fn vehicle_types_identify_the_e_bikes() {
    let feed = VehicleTypes::from_slice(&fixture("vehicle_types")).unwrap();
    let e_bike = feed.data.get("3645").unwrap();

    assert_eq!(e_bike.form_factor, FormFactor::Bicycle);
    assert_eq!(e_bike.propulsion_type, Propulsion::ElectricAssist);
    assert_eq!(e_bike.max_range_meters, Some(50_000.0));
    assert_eq!(
        feed.data.get("3646").unwrap().propulsion_type,
        Propulsion::Human
    );
}

#[test]
fn zones_have_the_hashes_that_the_collector_records() {
    // The moby-collector hashes of the first two zones of the feed of 27
    // September 2026, which are the same zones as in the fixture.
    let feed = GeofencingZones::from_slice(&fixture("geofencing_zones")).unwrap();
    let hashes: Vec<&str> = feed
        .data
        .geofencing_zones
        .features
        .iter()
        .map(|zone| zone.hash.as_str())
        .collect();

    assert_eq!(
        hashes[..2],
        [
            "30b522cc6e5091aeaa19275e0466b1fcf8f29fd7227067cd419a3df499437537",
            "f672356c2566a4da29ebb74f438ebc2ff29e2bfe509938741737f2f38affd0c3",
        ]
    );
    let unique: std::collections::HashSet<&str> = hashes.iter().copied().collect();
    assert_eq!(unique.len(), hashes.len());
}

#[test]
fn zone_hash_does_not_depend_on_the_layout_of_the_body() {
    let compact: serde_json::Value = serde_json::from_str(
        r#"{"type":"Feature","properties":{"rules":[]},"geometry":{"type":"MultiPolygon","coordinates":[]}}"#,
    )
    .unwrap();
    let spaced: serde_json::Value = serde_json::from_str(
        r#"{ "geometry": { "coordinates": [], "type": "MultiPolygon" },
             "properties": { "rules": [] }, "type": "Feature" }"#,
    )
    .unwrap();

    assert_eq!(ZoneHash::of(&compact), ZoneHash::of(&spaced));
}

#[test]
fn zone_hash_changes_with_the_rules() {
    let feature = |end: bool| {
        serde_json::json!({
            "type": "Feature",
            "geometry": {"type": "MultiPolygon", "coordinates": []},
            "properties": {"rules": [{
                "ride_start_allowed": true,
                "ride_end_allowed": end,
                "ride_through_allowed": true,
            }]},
        })
    };

    assert_ne!(ZoneHash::of(&feature(true)), ZoneHash::of(&feature(false)));
}

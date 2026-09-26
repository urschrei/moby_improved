//! Tests for the vehicle index, against the linear scan in `candidates`.

use std::sync::Arc;

use hegel::TestCase;
use hegel::generators as gs;
use moby_core::Candidate;
use moby_core::Filter;
use moby_core::Position;
use moby_core::VehicleIndex;
use moby_core::candidates;
use moby_core::gbfs::RentalUris;
use moby_core::gbfs::Vehicle;
use moby_core::gbfs::VehicleStatus;

/// The Spire on O'Connell Street.
const SPIRE: Position = Position {
    lat: 53.349_805,
    lon: -6.260_31,
};

/// The tolerance for distances that the index and the scan calculate in a
/// different way.
const TOLERANCE_M: f64 = 1e-6;

fn draw_position(tc: &TestCase) -> Position {
    Position::new(
        tc.draw(gs::floats::<f64>().min_value(-90.0).max_value(90.0)),
        tc.draw(gs::floats::<f64>().min_value(-180.0).max_value(180.0)),
    )
}

/// Draws a position within about 10 km of the Spire.
fn draw_dublin_position(tc: &TestCase) -> Position {
    Position::new(
        tc.draw(gs::floats::<f64>().min_value(53.26).max_value(53.44)),
        tc.draw(gs::floats::<f64>().min_value(-6.41).max_value(-6.11)),
    )
}

/// Draws a vehicle. Some vehicles share the position of an earlier vehicle,
/// as bikes in one bay do.
fn draw_vehicle(tc: &TestCase, index: usize, earlier: &[Vehicle]) -> Vehicle {
    let shared = if earlier.is_empty() || !tc.draw(gs::booleans()) {
        None
    } else {
        let other = &earlier[tc.draw(gs::integers::<usize>().max_value(earlier.len() - 1))];
        other
            .lat
            .zip(other.lon)
            .map(|(lat, lon)| Position::new(lat, lon))
    };
    let position = shared.or_else(|| tc.draw(gs::booleans()).then(|| draw_dublin_position(tc)));
    Vehicle {
        vehicle_id: index.to_string(),
        lat: position.map(|position| position.lat),
        lon: position.map(|position| position.lon),
        is_reserved: tc.draw(gs::booleans()),
        is_disabled: tc.draw(gs::booleans()),
        vehicle_type_id: Some("3645".to_owned()),
        current_range_meters: tc.draw(gs::optional(
            gs::floats::<f64>().min_value(0.0).max_value(60_000.0),
        )),
        current_fuel_percent: None,
        last_reported: None,
        pricing_plan_id: None,
        rental_uris: Some(RentalUris {
            ios: Some(format!("https://moby-move.app.link/{index}")),
            ..RentalUris::default()
        }),
    }
}

fn draw_vehicles(tc: &TestCase) -> Vec<Vehicle> {
    let count = tc.draw(gs::integers::<usize>().max_value(60));
    let mut vehicles = Vec::with_capacity(count);
    for index in 0..count {
        let vehicle = draw_vehicle(tc, index, &vehicles);
        vehicles.push(vehicle);
    }
    vehicles
}

/// Draws an origin in Dublin, or anywhere.
fn draw_origin(tc: &TestCase) -> Position {
    if tc.draw(gs::booleans()) {
        draw_dublin_position(tc)
    } else {
        draw_position(tc)
    }
}

fn draw_filter(tc: &TestCase) -> Filter {
    Filter {
        min_range_m: tc.draw(gs::floats::<f64>().min_value(0.0).max_value(60_000.0)),
    }
}

fn sorted_by_id(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    candidates.sort_by(|a, b| a.vehicle_id.cmp(&b.vehicle_id));
    candidates
}

fn feed() -> VehicleStatus {
    let body = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/vehicle_status.json"
    ))
    .unwrap();
    VehicleStatus::from_slice(&body).unwrap().data
}

#[hegel::test]
fn nearest_finds_the_same_candidates_as_the_scan(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let origin = draw_origin(&tc);
    let filter = draw_filter(&tc);

    let found: Vec<Candidate> = VehicleIndex::new(&vehicles)
        .nearest(origin, filter)
        .collect();

    assert_eq!(
        sorted_by_id(found),
        sorted_by_id(candidates(&vehicles, origin, filter))
    );
}

#[hegel::test]
fn nearest_is_in_straight_line_order(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let origin = draw_origin(&tc);
    let index = VehicleIndex::new(&vehicles);

    let distances: Vec<f64> = index
        .nearest(origin, draw_filter(&tc))
        .map(|candidate| candidate.straight_line_m)
        .collect();

    for pair in distances.windows(2) {
        assert!(pair[0] <= pair[1] + TOLERANCE_M, "{pair:?}");
    }
}

#[hegel::test]
fn shared_nearest_matches_nearest(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let origin = draw_origin(&tc);
    let filter = draw_filter(&tc);
    let index = Arc::new(VehicleIndex::new(&vehicles));

    let shared: Vec<Candidate> = index.shared_nearest(origin, filter).collect();

    assert_eq!(shared, index.nearest(origin, filter).collect::<Vec<_>>());
}

#[hegel::test]
fn index_holds_the_available_vehicles_with_a_position_and_a_range(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let expected = vehicles
        .iter()
        .filter(|vehicle| {
            vehicle.is_available()
                && vehicle.position().is_some()
                && vehicle.current_range_meters.is_some()
        })
        .count();

    let index = VehicleIndex::new(&vehicles);

    assert_eq!(index.len(), expected);
    assert_eq!(index.is_empty(), expected == 0);
}

#[hegel::test]
fn taking_a_prefix_does_not_change_its_order(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let origin = draw_origin(&tc);
    let filter = draw_filter(&tc);
    let index = VehicleIndex::new(&vehicles);
    let all: Vec<Candidate> = index.nearest(origin, filter).collect();
    let count = tc.draw(gs::integers::<usize>().max_value(all.len()));

    let prefix: Vec<Candidate> = index.nearest(origin, filter).take(count).collect();

    assert_eq!(prefix, all[..count]);
}

#[test]
fn nearest_matches_the_scan_on_the_live_feed() {
    let feed = feed();
    let filter = Filter {
        min_range_m: 10_000.0,
    };
    let index = VehicleIndex::new(&feed.vehicles);

    let found: Vec<Candidate> = index.nearest(SPIRE, filter).collect();
    let scanned = candidates(&feed.vehicles, SPIRE, filter);

    assert_eq!(found.len(), scanned.len());
    for (found, scanned) in found.iter().zip(&scanned) {
        assert!((found.straight_line_m - scanned.straight_line_m).abs() < TOLERANCE_M);
    }
    assert_eq!(sorted_by_id(found), sorted_by_id(scanned));
}

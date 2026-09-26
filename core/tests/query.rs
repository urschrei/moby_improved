//! Tests for distance, candidate selection and feed freshness.

use hegel::TestCase;
use hegel::generators as gs;
use jiff::SignedDuration;
use jiff::Timestamp;
use moby_core::Filter;
use moby_core::Freshness;
use moby_core::Position;
use moby_core::candidates;
use moby_core::gbfs::RentalUris;
use moby_core::gbfs::Vehicle;
use moby_core::gbfs::VehicleStatus;

/// The Spire on O'Connell Street.
const SPIRE: Position = Position {
    lat: 53.349_805,
    lon: -6.260_31,
};

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

fn draw_vehicle(tc: &TestCase, index: usize) -> Vehicle {
    let position = tc.draw(gs::booleans()).then(|| draw_dublin_position(tc));
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
    let count = tc.draw(gs::integers::<usize>().max_value(40));
    (0..count).map(|index| draw_vehicle(tc, index)).collect()
}

fn draw_filter(tc: &TestCase) -> Filter {
    Filter {
        min_range_m: tc.draw(gs::floats::<f64>().min_value(0.0).max_value(60_000.0)),
    }
}

fn satisfies(vehicle: &Vehicle, filter: Filter) -> bool {
    vehicle.is_available()
        && vehicle.position().is_some()
        && vehicle
            .current_range_meters
            .is_some_and(|range_m| range_m >= filter.min_range_m)
}

#[test]
fn one_degree_of_longitude_at_the_equator() {
    let distance_m = Position::new(0.0, 0.0).distance_m(Position::new(0.0, 1.0));

    assert!((distance_m - 111_195.08).abs() < 0.1, "{distance_m}");
}

#[test]
fn spire_to_heuston_station() {
    let heuston = Position::new(53.346_4, -6.292_3);
    let distance_m = SPIRE.distance_m(heuston);

    assert!((2_100.0..2_200.0).contains(&distance_m), "{distance_m}");
}

#[hegel::test]
fn distance_is_symmetric(tc: TestCase) {
    let a = draw_position(&tc);
    let b = draw_position(&tc);

    assert!((a.distance_m(b) - b.distance_m(a)).abs() < 1e-6);
}

#[hegel::test]
fn distance_is_between_zero_and_half_the_circumference(tc: TestCase) {
    let distance_m = draw_position(&tc).distance_m(draw_position(&tc));

    assert!((0.0..=20_015_200.0).contains(&distance_m), "{distance_m}");
}

#[hegel::test]
fn distance_obeys_the_triangle_inequality(tc: TestCase) {
    let a = draw_dublin_position(&tc);
    let b = draw_dublin_position(&tc);
    let c = draw_dublin_position(&tc);

    assert!(a.distance_m(c) <= a.distance_m(b) + b.distance_m(c) + 1e-6);
}

#[hegel::test]
fn candidates_are_sorted_by_straight_line_distance(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let found = candidates(&vehicles, draw_dublin_position(&tc), draw_filter(&tc));

    assert!(
        found
            .windows(2)
            .all(|pair| pair[0].straight_line_m <= pair[1].straight_line_m)
    );
}

#[hegel::test]
fn candidates_are_exactly_the_vehicles_that_satisfy_the_filter(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let filter = draw_filter(&tc);
    let found = candidates(&vehicles, draw_dublin_position(&tc), filter);

    let mut found_ids: Vec<&str> = found.iter().map(|c| c.vehicle_id.as_str()).collect();
    let mut expected_ids: Vec<&str> = vehicles
        .iter()
        .filter(|vehicle| satisfies(vehicle, filter))
        .map(|vehicle| vehicle.vehicle_id.as_str())
        .collect();
    found_ids.sort_unstable();
    expected_ids.sort_unstable();
    assert_eq!(found_ids, expected_ids);
}

#[hegel::test]
fn candidate_distance_is_the_distance_from_the_origin(tc: TestCase) {
    let vehicles = draw_vehicles(&tc);
    let origin = draw_dublin_position(&tc);

    for candidate in candidates(&vehicles, origin, draw_filter(&tc)) {
        assert_eq!(
            candidate.straight_line_m.to_bits(),
            origin.distance_m(candidate.position).to_bits()
        );
    }
}

#[test]
fn fixture_candidates_near_the_spire() {
    let body = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/vehicle_status.json"
    ))
    .unwrap();
    let feed = VehicleStatus::from_slice(&body).unwrap();
    let filter = Filter {
        min_range_m: 10_000.0,
    };
    let found = candidates(&feed.data.vehicles, SPIRE, filter);

    assert!(!found.is_empty());
    assert!(found.len() < feed.data.vehicles.len());
    assert!(
        found[0].straight_line_m < 500.0,
        "{}",
        found[0].straight_line_m
    );
    assert!(found.iter().all(|c| c.ios_rental_uri.is_some()));
}

#[hegel::test]
fn feed_is_stale_only_when_older_than_the_threshold(tc: TestCase) {
    let now = Timestamp::from_second(1_790_000_000).unwrap();
    let age_s = tc.draw(gs::integers::<i64>().min_value(-3_600).max_value(86_400));
    let threshold_s = tc.draw(gs::integers::<i64>().min_value(0).max_value(3_600));
    let last_updated = now - SignedDuration::from_secs(age_s);
    let freshness = Freshness::measure(last_updated, now, SignedDuration::from_secs(threshold_s));

    assert_eq!(freshness.age, SignedDuration::from_secs(age_s.max(0)));
    assert_eq!(freshness.is_stale, age_s > threshold_s);
}

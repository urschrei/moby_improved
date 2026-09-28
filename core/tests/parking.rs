//! Tests for the index of parking bays.

use std::sync::LazyLock;

use geo::Closest;
use geo::Coord;
use geo::Distance as _;
use geo::Haversine;
use geo::HaversineClosestPoint as _;
use geo::LineString;
use geo::Point;
use geo::Polygon;
use hegel::TestCase;
use hegel::generators as gs;
use moby_core::Bay;
use moby_core::ParkingIndex;
use moby_core::Position;
use moby_core::gbfs::GeofencingZones;

const E_BIKE: &str = "3645";

static ZONES: LazyLock<GeofencingZones> = LazyLock::new(|| {
    let body = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/geofencing_zones.json"
    ))
    .unwrap();
    GeofencingZones::from_slice(&body).unwrap().data
});

static INDEX: LazyLock<ParkingIndex> = LazyLock::new(|| ParkingIndex::new(&ZONES, E_BIKE));

fn polygon(bay: &Bay) -> Polygon {
    Polygon::new(
        LineString::new(
            bay.outline
                .iter()
                .map(|position| Coord {
                    x: position.lon,
                    y: position.lat,
                })
                .collect(),
        ),
        Vec::new(),
    )
}

/// Returns the distance from `position` to the nearest bay, by checking
/// every bay.
fn brute_force_nearest_m(position: Position, bays: &[Bay]) -> f64 {
    let point = Point::new(position.lon, position.lat);
    bays.iter()
        .map(|bay| match polygon(bay).haversine_closest_point(&point) {
            Closest::Intersection(_) => 0.0,
            Closest::SinglePoint(closest) => Haversine.distance(point, closest),
            Closest::Indeterminate => f64::INFINITY,
        })
        .fold(f64::INFINITY, f64::min)
}

fn all_bays() -> Vec<Bay> {
    let everywhere = Position::new(53.35, -6.26);
    INDEX
        .nearest(everywhere, INDEX.len())
        .into_iter()
        .map(|nearby| nearby.bay)
        .collect()
}

fn draw_dublin_position(tc: &TestCase) -> Position {
    Position::new(
        tc.draw(gs::floats::<f64>().min_value(53.30).max_value(53.40)),
        tc.draw(gs::floats::<f64>().min_value(-6.33).max_value(-6.19)),
    )
}

#[test]
fn indexes_the_zones_where_an_e_bike_ride_can_end() {
    // 1,186 zones allow a ride to end; 2 of them are for pedal bikes only.
    assert_eq!(INDEX.len(), 1_184);
}

#[test]
fn each_bay_has_the_hash_of_its_zone() {
    let zones: std::collections::HashSet<&str> = ZONES
        .geofencing_zones
        .features
        .iter()
        .map(|zone| zone.hash.as_str())
        .collect();

    assert!(
        all_bays()
            .iter()
            .all(|bay| zones.contains(bay.zone_hash.as_str()))
    );
}

#[test]
fn a_point_inside_a_bay_is_at_distance_zero() {
    let bay = &all_bays()[0];

    let nearest = INDEX.nearest(bay.centroid, 1);

    assert_eq!(nearest[0].distance_m.to_bits(), 0.0_f64.to_bits());
}

#[test]
fn a_point_far_from_every_bay_is_not_inside_one() {
    // In Dublin Bay, several kilometres from the nearest bay. With clockwise
    // rings, every polygon would cover the rest of the sphere, and this
    // distance would be zero.
    let nearest = INDEX.nearest(Position::new(53.34, -6.05), 1);

    assert!(nearest[0].distance_m > 2_000.0, "{}", nearest[0].distance_m);
}

#[test]
fn bay_outlines_are_counter_clockwise() {
    use geo::Winding as _;

    assert!(
        all_bays()
            .iter()
            .all(|bay| polygon(bay).exterior().is_ccw())
    );
}

#[hegel::test(test_cases = 50)]
fn nearest_bay_matches_a_brute_force_search(tc: TestCase) {
    let position = draw_dublin_position(&tc);
    let nearest = INDEX.nearest(position, 1);
    let expected = brute_force_nearest_m(position, &all_bays());

    assert!(
        (nearest[0].distance_m - expected).abs() < 0.5,
        "index {} brute force {expected}",
        nearest[0].distance_m
    );
}

#[hegel::test]
fn nearest_bays_are_sorted_by_distance(tc: TestCase) {
    let position = draw_dublin_position(&tc);
    let k = tc.draw(gs::integers::<usize>().min_value(1).max_value(20));
    let nearest = INDEX.nearest(position, k);

    assert_eq!(nearest.len(), k);
    assert!(
        nearest
            .windows(2)
            .all(|pair| pair[0].distance_m <= pair[1].distance_m)
    );
}

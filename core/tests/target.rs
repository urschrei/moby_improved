//! Tests for tracking the vehicle that the rider is walking to.

use hegel::TestCase;
use hegel::generators as gs;
use moby_core::MOVE_THRESHOLD_M;
use moby_core::Position;
use moby_core::TargetStatus;
use moby_core::gbfs::Vehicle;
use moby_core::gbfs::VehicleStatus;
use moby_core::target_status;

fn snapshot(time: &str) -> Vec<Vehicle> {
    let path = format!(
        "{}/tests/fixtures/probe/vehicle_status_{time}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    VehicleStatus::from_slice(&std::fs::read(path).unwrap())
        .unwrap()
        .data
        .vehicles
}

fn position_of(vehicles: &[Vehicle], vehicle_id: &str) -> Position {
    let (lat, lon) = vehicles
        .iter()
        .find(|vehicle| vehicle.vehicle_id == vehicle_id)
        .and_then(Vehicle::position)
        .unwrap();
    Position::new(lat, lon)
}

fn vehicle(lat: f64, lon: f64, is_reserved: bool, is_disabled: bool) -> Vehicle {
    Vehicle {
        vehicle_id: "target".to_owned(),
        lat: Some(lat),
        lon: Some(lon),
        is_reserved,
        is_disabled,
        vehicle_type_id: None,
        current_range_meters: Some(20_000.0),
        current_fuel_percent: None,
        last_reported: None,
        pricing_plan_id: None,
        rental_uris: None,
    }
}

#[test]
fn a_rented_vehicle_is_gone_from_the_next_response() {
    let before = snapshot("1118");
    let after = snapshot("1121");
    let chosen_at = position_of(&before, "4120848858822619701");

    assert_eq!(
        target_status(&after, "4120848858822619701", chosen_at),
        TargetStatus::Gone
    );
}

#[test]
fn most_vehicles_are_still_available_three_minutes_later() {
    let before = snapshot("1118");
    let after = snapshot("1121");
    let available = before
        .iter()
        .filter(|vehicle| {
            let (lat, lon) = vehicle.position().unwrap();
            target_status(&after, &vehicle.vehicle_id, Position::new(lat, lon))
                == TargetStatus::Available
        })
        .count();

    assert!(
        available > before.len() * 9 / 10,
        "{available} of {}",
        before.len()
    );
}

#[hegel::test]
fn status_follows_availability_and_distance(tc: TestCase) {
    let chosen_at = Position::new(53.35, -6.26);
    // Up to about 110 m north of the chosen position.
    let dlat = tc.draw(gs::floats::<f64>().min_value(0.0).max_value(0.001));
    let is_reserved = tc.draw(gs::booleans());
    let is_disabled = tc.draw(gs::booleans());
    let current = vehicle(
        chosen_at.lat + dlat,
        chosen_at.lon,
        is_reserved,
        is_disabled,
    );
    let moved_m = chosen_at.distance_m(Position::new(chosen_at.lat + dlat, chosen_at.lon));

    let expected = if is_reserved || is_disabled {
        TargetStatus::Gone
    } else if moved_m > MOVE_THRESHOLD_M {
        TargetStatus::Moved(Position::new(chosen_at.lat + dlat, chosen_at.lon))
    } else {
        TargetStatus::Available
    };
    assert_eq!(target_status(&[current], "target", chosen_at), expected);
}

#[hegel::test]
fn a_vehicle_that_is_not_listed_is_gone(tc: TestCase) {
    let count = tc.draw(gs::integers::<usize>().max_value(5));
    let others: Vec<Vehicle> = (0..count)
        .map(|index| Vehicle {
            vehicle_id: format!("other-{index}"),
            ..vehicle(53.35, -6.26, false, false)
        })
        .collect();

    assert_eq!(
        target_status(&others, "target", Position::new(53.35, -6.26)),
        TargetStatus::Gone
    );
}

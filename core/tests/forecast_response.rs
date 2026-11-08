//! Tests of the forecast response of the collector's service.
//!
//! `response_work.json` is a response of the service on 2026-11-08, for
//! targets 30 min, 1 h, 2 h and 6 h after the request.

use hegel::TestCase;
use hegel::generators as gs;
use jiff::SignedDuration;
use jiff::Timestamp;
use moby_core::Error;
use moby_core::forecast::Response;
use moby_core::forecast::Target;
use moby_core::forecast::request_targets;

fn fixture() -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/forecast/response_work.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

fn target(probability: f64, horizon: &str, skill: Option<f64>) -> Target {
    Target {
        at: "2026-11-08T10:00:00Z".parse().unwrap(),
        probability,
        horizon: horizon.to_owned(),
        skill,
    }
}

#[test]
fn a_response_of_the_service_parses() {
    let response = Response::from_slice(&fixture()).unwrap();

    assert_eq!(response.place, "work");
    assert_eq!((response.certain, response.suspect), (0, 0));
    assert_eq!(response.model.mode, "bike");
    let horizons: Vec<_> = response
        .targets
        .iter()
        .map(|t| t.horizon.as_str())
        .collect();
    assert_eq!(horizons, ["60 min", "2 h", "6 h", "overnight"]);
}

#[test]
fn the_app_shows_short_horizons_with_skill() {
    let response = Response::from_slice(&fixture()).unwrap();

    let shown: Vec<_> = response.targets.iter().map(Target::is_shown).collect();

    // The skill at 6 h and overnight is below 0.
    assert_eq!(shown, [true, true, false, false]);
}

#[test]
fn a_short_horizon_without_skill_is_not_shown() {
    assert!(!target(0.5, "30 min", Some(0.0)).is_shown());
    assert!(!target(0.5, "30 min", Some(-0.1)).is_shown());
    assert!(!target(0.5, "30 min", None).is_shown());
    assert!(!target(0.5, "6 h", Some(0.2)).is_shown());
}

#[test]
fn percentages_are_rounded_to_5() {
    let response = Response::from_slice(&fixture()).unwrap();

    let percent: Vec<_> = response.targets.iter().map(Target::percent).collect();

    assert_eq!(percent, [25, 45, 65, 85]);
}

#[test]
fn percentages_stop_short_of_0_and_100() {
    assert_eq!(target(0.0, "5 min", Some(0.8)).percent(), 5);
    assert_eq!(target(1.0, "5 min", Some(0.8)).percent(), 95);
}

#[test]
fn unknown_fields_are_ignored() {
    let mut value: serde_json::Value = serde_json::from_slice(&fixture()).unwrap();
    value["note"] = "a later field".into();
    value["targets"][0]["interval"] = serde_json::json!([0.2, 0.3]);

    let response = Response::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();

    assert_eq!(response.targets.len(), 4);
}

#[test]
fn an_error_body_is_not_a_forecast() {
    let result = Response::from_slice(br#"{"error":"unknown place \"nowhere\""}"#);

    let Err(Error::Forecast(_)) = result else {
        panic!("expected a parse error, got {result:?}");
    };
}

#[test]
fn a_probability_outside_0_to_1_is_refused() {
    let mut value: serde_json::Value = serde_json::from_slice(&fixture()).unwrap();
    value["targets"][1]["probability"] = 1.5.into();

    let result = Response::from_slice(&serde_json::to_vec(&value).unwrap());

    let Err(Error::OutOfRange { name, value }) = result else {
        panic!("expected a value out of range, got {result:?}");
    };
    assert_eq!(name, "probability");
    assert!((value - 1.5).abs() < 1e-12, "{value}");
}

#[test]
fn targets_are_requested_before_their_time() {
    let now: Timestamp = "2026-11-08T09:47:27Z".parse().unwrap();

    let targets = request_targets(now);

    let minutes: Vec<_> = targets
        .iter()
        .map(|at| at.duration_since(now).as_mins())
        .collect();
    assert_eq!(minutes, [28, 58, 118]);
}

#[hegel::test(test_cases = 200)]
fn a_percentage_is_a_step_of_5_near_the_probability(tc: TestCase) {
    let probability = tc.draw(gs::floats::<f64>().min_value(0.0).max_value(1.0));

    let percent = target(probability, "5 min", Some(0.8)).percent();

    assert_eq!(percent % 5, 0, "{percent}");
    assert!((5..=95).contains(&percent), "{percent}");
    let exact = (probability * 100.0).clamp(5.0, 95.0);
    assert!(
        (f64::from(percent) - exact).abs() <= 2.5,
        "{percent} for {probability}"
    );
}

#[hegel::test(test_cases = 50)]
fn requested_targets_are_in_order_and_ahead(tc: TestCase) {
    let now = Timestamp::UNIX_EPOCH
        + SignedDuration::from_secs(
            tc.draw(gs::integers::<i64>().min_value(0).max_value(4_000_000_000)),
        );

    let targets = request_targets(now);

    assert!(targets[0] > now);
    assert!(targets.windows(2).all(|pair| pair[0] < pair[1]));
}

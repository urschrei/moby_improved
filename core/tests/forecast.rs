//! Tests of the availability forecast against the Python model.
//!
//! The fixtures in `tests/fixtures/forecast` are written by
//! `analysis/export_fixtures.py` in the `moby_analysis` repository, from
//! synthetic statistics.

use hegel::TestCase;
use hegel::generators as gs;
use jiff::SignedDuration;
use jiff::Timestamp;
use moby_core::Error;
use moby_core::forecast::Chain;
use moby_core::forecast::Mode;
use moby_core::forecast::Parameters;
use serde::Deserialize;

/// A case from `cases.json`, with the result of the Python model.
#[derive(Deserialize)]
struct Case {
    name: String,
    parameters: String,
    place: Option<String>,
    bays: Option<Vec<String>>,
    certain: usize,
    suspect: usize,
    origin: Timestamp,
    targets: Vec<Timestamp>,
    birth: Vec<f64>,
    death: Vec<f64>,
    probabilities: Vec<f64>,
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/forecast/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {path}: {error}"))
}

fn parameters(name: &str) -> Parameters {
    Parameters::from_slice(&fixture(name)).unwrap()
}

fn cases() -> Vec<Case> {
    serde_json::from_slice(&fixture("cases.json")).unwrap()
}

fn chain(case: &Case) -> Chain {
    let parameters = parameters(&case.parameters);
    match (&case.place, &case.bays) {
        (Some(place), _) => parameters.place(place).unwrap(),
        (None, Some(bays)) => parameters.reach(bays),
        (None, None) => panic!("{}: no place and no bays", case.name),
    }
}

fn assert_close(actual: &[f64], expected: &[f64], tolerance: f64, what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}");
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a - e).abs() <= tolerance * e.abs().max(1.0),
            "{what} [{i}]: {a} against {e}"
        );
    }
}

#[test]
fn parameter_files_conform_to_the_schema() {
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/schemas/forecast_parameters.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for name in ["parameters_pool.json", "parameters_bike.json"] {
        let instance: serde_json::Value = serde_json::from_slice(&fixture(name)).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|error| format!("{}: {error}", error.instance_path()))
            .collect();
        assert_eq!(errors, Vec::<String>::new(), "{name}");
    }
}

#[test]
fn parameters_give_the_mode_and_the_places() {
    let pool = parameters("parameters_pool.json");
    let bike = parameters("parameters_bike.json");

    assert_eq!(pool.mode, Mode::Pool);
    assert_eq!(bike.mode, Mode::Bike);
    assert_eq!(pool.places.keys().collect::<Vec<_>>(), ["a", "b", "c"]);
    assert!(pool.place("nowhere").is_none());
    assert!((pool.suspect_share - 0.68).abs() < 1e-12);
    assert!((pool.rules.min_range_m - 10_000.0).abs() < 1e-9);
}

#[test]
fn rates_of_a_reach_agree_with_the_python_model() {
    for case in cases() {
        let chain = chain(&case);

        assert_close(chain.birth(), &case.birth, 1e-12, &case.name);
        assert_close(chain.death(), &case.death, 1e-12, &case.name);
    }
}

#[test]
fn availability_agrees_with_the_python_model() {
    for case in cases() {
        let mut chain = chain(&case);

        let probabilities =
            chain.availability(case.certain, case.suspect, case.origin, &case.targets);

        assert_close(&probabilities, &case.probabilities, 1e-12, &case.name);
    }
}

#[test]
fn targets_in_any_order_give_the_same_probabilities() {
    let case = &cases()[0];
    let mut reversed = case.targets.clone();
    reversed.reverse();

    let probabilities =
        chain(case).availability(case.certain, case.suspect, case.origin, &reversed);

    let mut expected = case.probabilities.clone();
    expected.reverse();
    assert_close(&probabilities, &expected, 1e-12, &case.name);
}

#[test]
fn a_target_before_the_origin_has_the_probability_at_the_origin() {
    let case = &cases()[0];
    let before = case.origin - SignedDuration::from_mins(10);

    let empty = chain(case).availability(0, 0, case.origin, &[before]);
    let one = chain(case).availability(1, 0, case.origin, &[before]);

    assert_close(&empty, &[0.0], 0.0, "empty");
    assert_close(&one, &[1.0], 0.0, "one bike");
}

#[test]
fn an_unknown_format_is_refused() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fixture("parameters_pool.json")).unwrap();
    value["format"] = 2.into();
    value["a field of format 2"] = true.into();

    let error = Parameters::from_slice(value.to_string().as_bytes()).unwrap_err();

    assert!(matches!(error, Error::UnknownFormat(2)), "{error}");
}

#[test]
fn a_profile_of_the_wrong_length_is_refused() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fixture("parameters_pool.json")).unwrap();
    value["arrival"]["profile"] = serde_json::json!([0.1, 0.2]);

    let error = Parameters::from_slice(value.to_string().as_bytes()).unwrap_err();

    assert!(matches!(error, Error::ProfileLength(2)), "{error}");
}

#[test]
fn an_unknown_field_is_refused() {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fixture("parameters_pool.json")).unwrap();
    value["rules"]["new_rule"] = 1.into();

    let error = Parameters::from_slice(value.to_string().as_bytes()).unwrap_err();

    assert!(matches!(error, Error::Parameters(_)), "{error}");
}

fn refused(change: impl Fn(&mut serde_json::Value)) -> Error {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fixture("parameters_bike.json")).unwrap();
    change(&mut value);
    Parameters::from_slice(value.to_string().as_bytes()).unwrap_err()
}

#[test]
fn values_out_of_range_are_refused() {
    let share = refused(|value| value["suspect_share"] = 1.5.into());
    let strength = refused(|value| value["departure"]["strength"] = 0.0.into());
    let rate = refused(|value| value["arrival"]["profile"][3] = (-0.1).into());
    let states = refused(|value| value["min_states"] = 1.into());

    for (error, expected) in [
        (share, "suspect_share"),
        (strength, "strength"),
        (rate, "profile"),
        (states, "min_states"),
    ] {
        assert!(
            matches!(error, Error::OutOfRange { name, .. } if name == expected),
            "{error}"
        );
    }
}

#[test]
fn another_time_zone_is_refused() {
    let error = refused(|value| value["timezone"] = "Asia/Kolkata".into());

    assert!(matches!(error, Error::UnsupportedTimeZone(_)), "{error}");
}

#[test]
fn a_malformed_zone_hash_is_refused() {
    let error = refused(|value| value["places"]["a"]["bays"][0] = "ABC".into());

    assert!(matches!(error, Error::Parameters(_)), "{error}");
}

#[test]
fn a_reach_with_no_bays_has_no_births_and_no_deaths() {
    for name in ["parameters_pool.json", "parameters_bike.json"] {
        let chain = parameters(name).reach::<&str>(&[]);

        assert!(chain.birth().iter().all(|&rate| rate == 0.0), "{name}");
        assert!(chain.death().iter().all(|&rate| rate == 0.0), "{name}");
    }
}

fn draw_case(tc: &TestCase) -> (Chain, usize, usize, Timestamp, Timestamp) {
    let name = if tc.draw(gs::booleans()) {
        "parameters_pool.json"
    } else {
        "parameters_bike.json"
    };
    let place = ["a", "b", "c"][tc.draw(gs::integers::<usize>().min_value(0).max_value(2))];
    let certain = tc.draw(gs::integers::<usize>().min_value(0).max_value(30));
    let suspect = tc.draw(gs::integers::<usize>().min_value(0).max_value(5));
    // Any minute of October 2026, and a horizon of up to 16 hours.
    let origin = Timestamp::from_second(1_790_812_800).unwrap()
        + SignedDuration::from_mins(tc.draw(gs::integers::<i64>().min_value(0).max_value(44_640)));
    let target = origin
        + SignedDuration::from_mins(tc.draw(gs::integers::<i64>().min_value(0).max_value(960)));
    (
        parameters(name).place(place).unwrap(),
        certain,
        suspect,
        origin,
        target,
    )
}

#[hegel::test(test_cases = 50)]
fn availability_is_a_probability(tc: TestCase) {
    let (mut chain, certain, suspect, origin, target) = draw_case(&tc);

    let p = chain.availability(certain, suspect, origin, &[target])[0];

    assert!((0.0..=1.0 + 1e-12).contains(&p), "{p}");
}

#[hegel::test(test_cases = 50)]
fn more_bikes_at_the_origin_do_not_lower_the_availability(tc: TestCase) {
    // A birth-death chain is monotone in its initial state.
    let (mut chain, certain, _, origin, target) = draw_case(&tc);

    let fewer = chain.availability(certain, 0, origin, &[target])[0];
    let more = chain.availability(certain + 1, 0, origin, &[target])[0];

    assert!(more >= fewer - 1e-9, "{more} < {fewer}");
}

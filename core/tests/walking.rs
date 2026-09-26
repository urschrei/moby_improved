//! Tests for the walking-distance search and its cache.

use std::collections::HashMap;
use std::collections::HashSet;

use hegel::TestCase;
use hegel::generators as gs;
use moby_core::Candidate;
use moby_core::Position;
use moby_core::Route;
use moby_core::WalkCache;
use moby_core::WalkingSearch;

const ORIGIN: Position = Position {
    lat: 53.349_805,
    lon: -6.260_31,
};

/// A candidate list in straight-line order, with the walking route to each
/// candidate. No route is shorter than the straight line.
struct World {
    candidates: Vec<Candidate>,
    routes: HashMap<String, Route>,
}

fn candidate(index: usize, straight_line_m: f64) -> Candidate {
    Candidate {
        vehicle_id: index.to_string(),
        // Put each vehicle in its own cache cell.
        position: Position::new(
            ORIGIN.lat + 0.001 * f64::from(u32::try_from(index).unwrap()),
            ORIGIN.lon,
        ),
        range_m: 20_000.0,
        straight_line_m,
        ios_rental_uri: None,
    }
}

fn walk(distance_m: f64) -> Route {
    Route {
        distance_m,
        duration_s: distance_m / 1.4,
    }
}

fn draw_world(tc: &TestCase) -> World {
    let mut straight_lines: Vec<f64> =
        tc.draw(gs::vecs(gs::floats::<f64>().min_value(0.0).max_value(3_000.0)).max_size(30));
    straight_lines.sort_by(f64::total_cmp);
    let candidates: Vec<Candidate> = straight_lines
        .iter()
        .enumerate()
        .map(|(index, &straight_line_m)| candidate(index, straight_line_m))
        .collect();
    let routes = candidates
        .iter()
        .map(|candidate| {
            let detour = tc.draw(gs::floats::<f64>().min_value(1.0).max_value(3.0));
            (
                candidate.vehicle_id.clone(),
                walk(candidate.straight_line_m * detour),
            )
        })
        .collect();
    World { candidates, routes }
}

/// Runs a search to completion. Each step either issues a request or
/// completes one of the requests in flight, in an order that the test case
/// chooses. Returns the identifiers of the vehicles requested, in order.
fn drive(
    tc: &TestCase,
    search: &mut WalkingSearch,
    routes: &HashMap<String, Route>,
    failures: &HashSet<String>,
) -> Vec<String> {
    let mut requested = Vec::new();
    let mut in_flight: Vec<Candidate> = Vec::new();
    loop {
        if (in_flight.is_empty() || tc.draw(gs::booleans()))
            && let Some(candidate) = search.next_request()
        {
            requested.push(candidate.vehicle_id.clone());
            in_flight.push(candidate);
        } else if in_flight.is_empty() {
            return requested;
        } else {
            let index = tc.draw(gs::integers::<usize>().max_value(in_flight.len() - 1));
            let done = in_flight.swap_remove(index);
            if failures.contains(&done.vehicle_id) {
                search.report_failure(&done.vehicle_id);
            } else {
                search.report_route(&done.vehicle_id, routes[&done.vehicle_id]);
            }
        }
    }
}

fn distances(routes: impl IntoIterator<Item = Route>) -> Vec<u64> {
    let mut distances: Vec<f64> = routes.into_iter().map(|route| route.distance_m).collect();
    distances.sort_by(f64::total_cmp);
    distances.into_iter().map(f64::to_bits).collect()
}

#[hegel::test]
fn search_finds_the_k_shortest_walks(tc: TestCase) {
    let world = draw_world(&tc);
    let k = tc.draw(gs::integers::<usize>().min_value(1).max_value(6));
    let mut search = WalkingSearch::new(
        ORIGIN,
        world.candidates.clone(),
        k,
        usize::MAX,
        &WalkCache::default(),
    );
    drive(&tc, &mut search, &world.routes, &HashSet::new());

    let mut expected = distances(world.routes.values().copied());
    expected.truncate(k);
    assert_eq!(
        distances(search.results().iter().map(|ranked| ranked.route)),
        expected
    );
}

#[hegel::test]
fn search_is_complete_when_the_driver_stops(tc: TestCase) {
    let world = draw_world(&tc);
    let k = tc.draw(gs::integers::<usize>().min_value(1).max_value(6));
    let max_requests = tc.draw(gs::integers::<usize>().max_value(40));
    let mut search = WalkingSearch::new(
        ORIGIN,
        world.candidates,
        k,
        max_requests,
        &WalkCache::default(),
    );
    drive(&tc, &mut search, &world.routes, &HashSet::new());

    assert!(search.is_complete());
}

#[hegel::test]
fn search_requests_each_vehicle_once_within_the_budget(tc: TestCase) {
    let world = draw_world(&tc);
    let k = tc.draw(gs::integers::<usize>().min_value(1).max_value(6));
    let max_requests = tc.draw(gs::integers::<usize>().max_value(40));
    let mut search = WalkingSearch::new(
        ORIGIN,
        world.candidates,
        k,
        max_requests,
        &WalkCache::default(),
    );
    let requested = drive(&tc, &mut search, &world.routes, &HashSet::new());

    let unique: HashSet<&String> = requested.iter().collect();
    assert_eq!(unique.len(), requested.len());
    assert!(requested.len() <= max_requests);
    assert_eq!(search.requests(), requested.len());
}

#[hegel::test]
fn search_returns_k_results_when_there_are_k_candidates(tc: TestCase) {
    let world = draw_world(&tc);
    let k = tc.draw(gs::integers::<usize>().min_value(1).max_value(6));
    let count = world.candidates.len();
    let mut search = WalkingSearch::new(
        ORIGIN,
        world.candidates,
        k,
        usize::MAX,
        &WalkCache::default(),
    );
    drive(&tc, &mut search, &world.routes, &HashSet::new());

    assert_eq!(search.results().len(), k.min(count));
}

#[hegel::test]
fn failed_routes_are_estimated(tc: TestCase) {
    let world = draw_world(&tc);
    let failures: HashSet<String> = world
        .candidates
        .iter()
        .filter(|_| tc.draw(gs::booleans()))
        .map(|candidate| candidate.vehicle_id.clone())
        .collect();
    let mut search = WalkingSearch::new(
        ORIGIN,
        world.candidates,
        3,
        usize::MAX,
        &WalkCache::default(),
    );
    drive(&tc, &mut search, &world.routes, &failures);

    for ranked in search.results() {
        assert_eq!(
            ranked.is_estimate,
            failures.contains(&ranked.candidate.vehicle_id)
        );
    }
}

#[hegel::test]
fn repeated_search_from_the_same_origin_uses_the_cache(tc: TestCase) {
    let world = draw_world(&tc);
    let k = tc.draw(gs::integers::<usize>().min_value(1).max_value(6));
    let mut cache = WalkCache::default();
    let mut first = WalkingSearch::new(ORIGIN, world.candidates.clone(), k, usize::MAX, &cache);
    drive(&tc, &mut first, &world.routes, &HashSet::new());
    cache.record(&first);

    let mut second = WalkingSearch::new(ORIGIN, world.candidates, k, usize::MAX, &cache);
    let requested = drive(&tc, &mut second, &world.routes, &HashSet::new());

    assert_eq!(requested, Vec::<String>::new());
    assert_eq!(second.results(), first.results());
}

#[test]
fn search_stops_when_no_candidate_can_be_nearer() {
    // Each walk equals its straight line, so the first three candidates are
    // the answer and the fourth candidate cannot be better.
    let candidates: Vec<Candidate> = (0..10)
        .map(|index| candidate(index, 100.0 * f64::from(u32::try_from(index).unwrap() + 1)))
        .collect();
    let mut search = WalkingSearch::new(ORIGIN, candidates, 3, usize::MAX, &WalkCache::default());

    let first_batch: Vec<Candidate> = std::iter::from_fn(|| search.next_request()).collect();
    assert_eq!(first_batch.len(), 3);
    assert!(!search.is_complete());
    for candidate in &first_batch {
        search.report_route(&candidate.vehicle_id, walk(candidate.straight_line_m));
    }

    assert_eq!(search.next_request(), None);
    assert!(search.is_complete());
    assert_eq!(search.requests(), 3);
}

#[test]
fn a_long_detour_makes_the_search_continue() {
    // The nearest vehicle is across the river: 100 m in a straight line,
    // 900 m on foot.
    let candidates = vec![
        candidate(0, 100.0),
        candidate(1, 300.0),
        candidate(2, 800.0),
    ];
    let mut search = WalkingSearch::new(ORIGIN, candidates, 1, usize::MAX, &WalkCache::default());

    let first = search.next_request().unwrap();
    search.report_route(&first.vehicle_id, walk(900.0));
    let second = search.next_request().unwrap();
    search.report_route(&second.vehicle_id, walk(350.0));

    assert_eq!(search.next_request(), None);
    assert!(search.is_complete());
    assert_eq!(search.results()[0].candidate.vehicle_id, "1");
}

#[test]
fn cache_misses_when_the_origin_moves() {
    let candidates = vec![candidate(0, 100.0)];
    let mut cache = WalkCache::default();
    let mut search = WalkingSearch::new(ORIGIN, candidates.clone(), 1, usize::MAX, &cache);
    let requested = search.next_request().unwrap();
    search.report_route(&requested.vehicle_id, walk(120.0));
    cache.record(&search);

    let moved = Position::new(ORIGIN.lat + 0.001, ORIGIN.lon);
    let mut search = WalkingSearch::new(moved, candidates, 1, usize::MAX, &cache);

    assert!(search.next_request().is_some());
}

#[test]
fn cache_removes_routes_that_searches_do_not_use() {
    let mut cache = WalkCache::default();
    let mut search = WalkingSearch::new(ORIGIN, vec![candidate(0, 100.0)], 1, usize::MAX, &cache);
    let requested = search.next_request().unwrap();
    search.report_route(&requested.vehicle_id, walk(120.0));
    cache.record(&search);
    assert_eq!(cache.len(), 1);

    let empty = WalkingSearch::new(ORIGIN, Vec::new(), 1, usize::MAX, &cache);
    for _ in 0..WalkCache::MAX_IDLE_SEARCHES {
        cache.record(&empty);
    }

    assert!(cache.is_empty());
}

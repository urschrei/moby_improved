use std::collections::HashMap;

use crate::Candidate;
use crate::Position;

/// A search for the `k` candidates with the shortest walking distance.
///
/// The host application does the routing. It calls
/// [`WalkingSearch::next_request`] to get the next candidate to route, and
/// reports each result with [`WalkingSearch::report_route`] or
/// [`WalkingSearch::report_failure`]. The host can have more than one request
/// in flight at a time.
///
/// The search asks for candidates in order of straight-line distance. It stops
/// when the straight-line distance of the next candidate is more than the
/// `k`-th shortest walking distance found. A walking route is never shorter
/// than the straight line, so no remaining candidate can be in the result.
#[derive(Clone, Debug)]
pub struct WalkingSearch {
    origin: Position,
    k: usize,
    max_requests: usize,
    candidates: Vec<Candidate>,
    slots: Vec<Slot>,
    next: usize,
    requests: usize,
}

/// A walking route from the origin to a vehicle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Route {
    /// The length of the route in metres.
    pub distance_m: f64,
    /// The expected walking time in seconds.
    pub duration_s: f64,
}

/// A candidate with its walking distance from the origin.
#[derive(Clone, Debug, PartialEq)]
pub struct RankedVehicle {
    /// The candidate.
    pub candidate: Candidate,
    /// The walking route to the candidate.
    pub route: Route,
    /// `true` if the router failed and `route` is an estimate from the
    /// straight-line distance.
    pub is_estimate: bool,
}

/// Walking routes from earlier searches.
///
/// A route is reused when neither the origin nor the vehicle has moved to a
/// different grid cell. Routes that no search has used for
/// [`WalkCache::MAX_IDLE_SEARCHES`] searches are removed.
#[derive(Clone, Debug)]
pub struct WalkCache {
    origin_cell_m: f64,
    vehicle_cell_m: f64,
    entries: HashMap<CacheKey, CacheEntry>,
    generation: u64,
}

/// The factor that converts a straight-line distance to an estimated walking
/// distance when the router fails.
pub const DETOUR_FACTOR: f64 = 1.3;

/// The walking speed in metres per second for estimated routes.
pub const WALKING_SPEED_M_S: f64 = 1.4;

impl WalkingSearch {
    /// Starts a search from `origin` over candidates in straight-line order,
    /// such as the output of [`crate::candidates`].
    ///
    /// Routes in `cache` are used without a request. The search issues no more
    /// than `max_requests` requests.
    #[must_use]
    pub fn new(
        origin: Position,
        candidates: Vec<Candidate>,
        k: usize,
        max_requests: usize,
        cache: &WalkCache,
    ) -> Self {
        let slots = candidates
            .iter()
            .map(|candidate| match cache.get(origin, candidate) {
                Some(route) => Slot::Walked(route),
                None => Slot::Unrouted,
            })
            .collect();
        Self {
            origin,
            k,
            max_requests,
            candidates,
            slots,
            next: 0,
            requests: 0,
        }
    }

    /// Returns the next candidate to route, or `None` if no request is useful
    /// now.
    ///
    /// `None` does not mean that the search is complete: a result for a
    /// request in flight can make another request useful. Use
    /// [`WalkingSearch::is_complete`] to find out.
    pub fn next_request(&mut self) -> Option<Candidate> {
        let index = self.peek()?;
        self.slots[index] = Slot::InFlight;
        self.requests += 1;
        self.next = index + 1;
        Some(self.candidates[index].clone())
    }

    /// Records the walking route to a vehicle.
    pub fn report_route(&mut self, vehicle_id: &str, route: Route) {
        if let Some(index) = self.in_flight_index(vehicle_id) {
            self.slots[index] = Slot::Walked(route);
        }
    }

    /// Records that the router could not find a route to a vehicle.
    pub fn report_failure(&mut self, vehicle_id: &str) {
        if let Some(index) = self.in_flight_index(vehicle_id) {
            self.slots[index] = Slot::Failed;
        }
    }

    /// Returns `true` if no request is in flight and no further request is
    /// useful.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.in_flight() == 0 && self.peek().is_none()
    }

    /// Returns the number of requests that the search issued.
    #[must_use]
    pub fn requests(&self) -> usize {
        self.requests
    }

    /// Returns the best `k` candidates with a route, shortest walk first.
    ///
    /// Candidates for which the router failed have an estimated route. If the
    /// search is not complete, the result can change.
    #[must_use]
    pub fn results(&self) -> Vec<RankedVehicle> {
        let mut ranked: Vec<RankedVehicle> = self
            .candidates
            .iter()
            .zip(&self.slots)
            .filter_map(|(candidate, slot)| match slot {
                Slot::Walked(route) => Some(RankedVehicle {
                    candidate: candidate.clone(),
                    route: *route,
                    is_estimate: false,
                }),
                Slot::Failed => Some(RankedVehicle {
                    candidate: candidate.clone(),
                    route: Route::estimate(candidate.straight_line_m),
                    is_estimate: true,
                }),
                Slot::Unrouted | Slot::InFlight => None,
            })
            .collect();
        ranked.sort_by(|a, b| a.route.distance_m.total_cmp(&b.route.distance_m));
        ranked.truncate(self.k);
        ranked
    }

    /// Returns the origin of the search.
    #[must_use]
    pub fn origin(&self) -> Position {
        self.origin
    }

    /// Returns the index of the next candidate to route, if a request is
    /// useful now.
    fn peek(&self) -> Option<usize> {
        if self.requests == self.max_requests {
            return None;
        }
        let index = (self.next..self.slots.len()).find(|&index| self.slots[index].is_unrouted())?;

        let bound_m = self.candidates[index].straight_line_m;
        let settled = self.settled_distances();
        let is_useful = if settled.len() >= self.k {
            settled[self.k - 1] > bound_m
        } else {
            settled.len() + self.in_flight() < self.k
        };
        is_useful.then_some(index)
    }

    fn settled_distances(&self) -> Vec<f64> {
        let mut distances: Vec<f64> = self
            .candidates
            .iter()
            .zip(&self.slots)
            .filter_map(|(candidate, slot)| match slot {
                Slot::Walked(route) => Some(route.distance_m),
                Slot::Failed => Some(Route::estimate(candidate.straight_line_m).distance_m),
                Slot::Unrouted | Slot::InFlight => None,
            })
            .collect();
        distances.sort_by(f64::total_cmp);
        distances
    }

    fn in_flight(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_in_flight()).count()
    }

    fn in_flight_index(&self, vehicle_id: &str) -> Option<usize> {
        self.candidates
            .iter()
            .zip(&self.slots)
            .position(|(candidate, slot)| candidate.vehicle_id == vehicle_id && slot.is_in_flight())
    }
}

impl Route {
    /// Estimates a walking route from a straight-line distance.
    #[must_use]
    pub fn estimate(straight_line_m: f64) -> Self {
        let distance_m = straight_line_m * DETOUR_FACTOR;
        Self {
            distance_m,
            duration_s: distance_m / WALKING_SPEED_M_S,
        }
    }
}

impl WalkCache {
    /// The number of searches after which an unused route is removed.
    pub const MAX_IDLE_SEARCHES: u64 = 20;

    /// Makes an empty cache with the given grid cell sizes in metres.
    #[must_use]
    pub fn new(origin_cell_m: f64, vehicle_cell_m: f64) -> Self {
        Self {
            origin_cell_m,
            vehicle_cell_m,
            entries: HashMap::new(),
            generation: 0,
        }
    }

    /// Stores the routes that a search found, and removes routes that no
    /// search has used for [`WalkCache::MAX_IDLE_SEARCHES`] searches.
    ///
    /// Estimated routes are not stored.
    pub fn record(&mut self, search: &WalkingSearch) {
        self.generation += 1;
        for (candidate, slot) in search.candidates.iter().zip(&search.slots) {
            if let Slot::Walked(route) = slot {
                let key = self.key(search.origin, candidate);
                self.entries.insert(
                    key,
                    CacheEntry {
                        route: *route,
                        generation: self.generation,
                    },
                );
            }
        }
        let oldest = self.generation.saturating_sub(Self::MAX_IDLE_SEARCHES);
        self.entries.retain(|_, entry| entry.generation > oldest);
    }

    /// Returns the number of routes in the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the cache contains no routes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn get(&self, origin: Position, candidate: &Candidate) -> Option<Route> {
        self.entries
            .get(&self.key(origin, candidate))
            .map(|entry| entry.route)
    }

    fn key(&self, origin: Position, candidate: &Candidate) -> CacheKey {
        CacheKey {
            vehicle_id: candidate.vehicle_id.clone(),
            origin_cell: Cell::of(origin, self.origin_cell_m),
            vehicle_cell: Cell::of(candidate.position, self.vehicle_cell_m),
        }
    }
}

impl Default for WalkCache {
    /// A cache with 25 m origin cells and 10 m vehicle cells.
    fn default() -> Self {
        Self::new(25.0, 10.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Slot {
    Unrouted,
    InFlight,
    Walked(Route),
    Failed,
}

impl Slot {
    fn is_unrouted(self) -> bool {
        match self {
            Self::Unrouted => true,
            Self::InFlight | Self::Walked(_) | Self::Failed => false,
        }
    }

    fn is_in_flight(self) -> bool {
        match self {
            Self::InFlight => true,
            Self::Unrouted | Self::Walked(_) | Self::Failed => false,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CacheKey {
    vehicle_id: String,
    origin_cell: Cell,
    vehicle_cell: Cell,
}

#[derive(Clone, Copy, Debug)]
struct CacheEntry {
    route: Route,
    generation: u64,
}

/// A cell in a grid of approximately square cells.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Cell {
    row: i64,
    column: i64,
}

impl Cell {
    /// The length in metres of one degree of latitude.
    const METRES_PER_DEGREE: f64 = 111_320.0;

    #[allow(clippy::cast_possible_truncation, reason = "cell indices are small")]
    fn of(position: Position, size_m: f64) -> Self {
        let lat_step = size_m / Self::METRES_PER_DEGREE;
        let row = (position.lat / lat_step).floor();
        // Use the latitude of the row centre so that every point in a row
        // uses the same longitude step.
        let row_lat = ((row + 0.5) * lat_step).to_radians();
        let lon_step = lat_step / row_lat.cos().max(1e-6);
        Self {
            row: row as i64,
            column: (position.lon / lon_step).floor() as i64,
        }
    }
}

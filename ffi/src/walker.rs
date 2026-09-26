use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use moby_core::Route;
use moby_core::SharedNearest;
use moby_core::WalkCache;
use moby_core::WalkingSearch;

use crate::Bike;
use crate::Coordinate;
use crate::Feed;
use crate::WalkedBike;

/// Finds the bikes with the shortest walk, with routes from the host's router.
///
/// Call [`Walker::start`], then take requests from [`Walker::next_request`]
/// and report each result, until [`Walker::is_complete`] returns `true`.
/// Then call [`Walker::finish`] to keep the routes for the next search.
#[derive(Debug, uniffi::Object)]
pub struct Walker {
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    cache: WalkCache,
    search: Option<WalkingSearch<SharedNearest>>,
}

#[uniffi::export]
impl Walker {
    /// Makes a walker with an empty route cache.
    #[uniffi::constructor]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                cache: WalkCache::default(),
                search: None,
            }),
        }
    }

    /// Starts a search for the `k` nearest bikes on foot from `origin`, over
    /// the bikes in `feed` with at least `min_range_m` of range.
    ///
    /// The search takes bikes from the feed's index only when it needs them.
    /// It replaces a search that is not finished.
    pub fn start(
        &self,
        origin: Coordinate,
        feed: &Arc<Feed>,
        min_range_m: f64,
        k: u32,
        max_requests: u32,
    ) {
        let mut state = self.lock();
        let search = WalkingSearch::new(
            origin.into(),
            feed.nearest(origin, min_range_m),
            k as usize,
            max_requests as usize,
            &state.cache,
        );
        state.search = Some(search);
    }

    /// Returns the next bike to route, or `None` if no request is useful now.
    pub fn next_request(&self) -> Option<Bike> {
        self.lock()
            .search
            .as_mut()
            .and_then(WalkingSearch::next_request)
            .map(Bike::from)
    }

    /// Records the walking route to a bike.
    pub fn report_route(&self, vehicle_id: &str, walking_m: f64, walking_s: f64) {
        if let Some(search) = self.lock().search.as_mut() {
            search.report_route(
                vehicle_id,
                Route {
                    distance_m: walking_m,
                    duration_s: walking_s,
                },
            );
        }
    }

    /// Records that routing to a bike failed.
    pub fn report_failure(&self, vehicle_id: &str) {
        if let Some(search) = self.lock().search.as_mut() {
            search.report_failure(vehicle_id);
        }
    }

    /// Returns `true` if the search needs no more routes.
    pub fn is_complete(&self) -> bool {
        self.lock()
            .search
            .as_mut()
            .is_none_or(WalkingSearch::is_complete)
    }

    /// Returns the number of bikes that the search took from the index.
    pub fn bikes_taken(&self) -> u32 {
        self.lock().search.as_ref().map_or(0, |search| {
            u32::try_from(search.candidates_taken()).unwrap_or(u32::MAX)
        })
    }

    /// Returns the best bikes found so far, shortest walk first.
    pub fn results(&self) -> Vec<WalkedBike> {
        self.lock()
            .search
            .as_ref()
            .map(|search| search.results().into_iter().map(Into::into).collect())
            .unwrap_or_default()
    }

    /// Ends the search and keeps its routes for later searches.
    pub fn finish(&self) {
        let mut state = self.lock();
        if let Some(search) = state.search.take() {
            state.cache.record(&search);
        }
    }
}

impl Default for Walker {
    fn default() -> Self {
        Self::new()
    }
}

impl Walker {
    fn lock(&self) -> MutexGuard<'_, State> {
        // A panic cannot leave the state inconsistent, so a poisoned lock is
        // safe to use.
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

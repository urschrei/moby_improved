use std::collections::HashMap;

use jiff::RoundMode;
use jiff::SignedDuration;
use jiff::Timestamp;
use jiff::TimestampRound;
use jiff::Unit;
use jiff::civil::Weekday;
use jiff::tz::TimeZone;

use super::parameters::Mode;

/// The number of time bins: 24 local hours on Monday to Friday, then 24 on
/// Saturday and Sunday.
pub const BINS: usize = 48;

/// The birth-death chain of the rentable bikes in the reach of a place.
///
/// Bikes arrive at the birth rate of the time bin. In [`Mode::Pool`], a bike
/// leaves at the death rate while the reach has at least one bike; in
/// [`Mode::Bike`], each bike leaves at the death rate. The chain for `n`
/// bikes at the origin has [`states_for`] states, and births stop at the
/// largest count.
///
/// The chain keeps the transition matrices that it computes, so a chain
/// that forecasts many origins is faster than a new chain for each. It keeps
/// at most [`MAX_CACHED`] of them.
#[derive(Clone, Debug)]
pub struct Chain {
    birth: [f64; BINS],
    death: [f64; BINS],
    mode: Mode,
    suspect_share: f64,
    min_states: usize,
    time_zone: TimeZone,
    cache: HashMap<(usize, i64, usize), Matrix>,
}

impl Chain {
    pub(crate) fn new(
        birth: &[f64; BINS],
        death: &[f64; BINS],
        mode: Mode,
        suspect_share: f64,
        min_states: usize,
        time_zone: TimeZone,
    ) -> Self {
        Self {
            birth: *birth,
            death: *death,
            mode,
            suspect_share,
            min_states,
            time_zone,
            cache: HashMap::new(),
        }
    }

    /// Returns the birth rate per minute in each time bin.
    #[must_use]
    pub fn birth(&self) -> &[f64; BINS] {
        &self.birth
    }

    /// Returns the death rate per minute in each time bin.
    #[must_use]
    pub fn death(&self) -> &[f64; BINS] {
        &self.death
    }

    /// Returns the probability of at least one rentable bike in the reach at
    /// each of `targets`, from `certain` bikes and `suspect` bikes at
    /// `origin`.
    ///
    /// Each suspect bike is rentable with the suspect share of the
    /// parameters. The result is in the order of `targets`. A target before
    /// the origin has the probability at the origin. The method takes
    /// `&mut self` only to cache transition matrices.
    #[must_use]
    pub fn availability(
        &mut self,
        certain: usize,
        suspect: usize,
        origin: Timestamp,
        targets: &[Timestamp],
    ) -> Vec<f64> {
        let mut result = vec![0.0; targets.len()];
        let mut order: Vec<usize> = (0..targets.len()).collect();
        order.sort_by_key(|&i| targets[i]);
        let states = states_for(certain + suspect, self.min_states);
        let mut p = initial(certain, suspect, self.suspect_share, states);
        let mut t = origin;
        for i in order {
            let target = targets[i].max(t);
            p = self.advance(p, t, target);
            t = target;
            result[i] = 1.0 - p[0];
        }
        result
    }

    /// Propagates the distribution `p` from `start` to `end`, one hour bin
    /// at a time.
    ///
    /// Irish local time differs from UTC by whole hours, so UTC hours are
    /// local hours; `Parameters::from_slice` refuses other time zones. Each part is rounded to whole minutes, with ties to
    /// even, as in the Python model.
    fn advance(&mut self, mut p: Vec<f64>, start: Timestamp, end: Timestamp) -> Vec<f64> {
        let mut t = start;
        while t < end {
            let boundary = (floor_hour(t) + SignedDuration::from_hours(1)).min(end);
            let minutes = (boundary.duration_since(t).as_secs_f64() / 60.0).round_ties_even();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a part is at most 60 minutes"
            )]
            let minutes = minutes as i64;
            if minutes > 0 {
                let bin = time_bin(t, &self.time_zone);
                p = self.step(bin, minutes, p.len()).left_multiply(&p);
            }
            t = boundary;
        }
        p
    }

    fn step(&mut self, bin: usize, minutes: i64, states: usize) -> &Matrix {
        let (birth, death, mode) = (self.birth[bin], self.death[bin], self.mode);
        if self.cache.len() >= MAX_CACHED && !self.cache.contains_key(&(bin, minutes, states)) {
            self.cache.clear();
        }
        self.cache
            .entry((bin, minutes, states))
            .or_insert_with(|| transition(birth, death, minutes, states, mode))
    }
}

/// The largest number of transition matrices that a chain keeps. A matrix of
/// 40 states takes 12.8 kB.
pub const MAX_CACHED: usize = 1_024;

/// Returns the number of states of the chain for `count` bikes at the origin:
/// `max(min_states, 2 * count + 10)`.
#[must_use]
pub(crate) fn states_for(count: usize, min_states: usize) -> usize {
    min_states.max(2 * count + 10)
}

/// Returns the time bin of `t`: the local hour in `time_zone`, plus 24 on
/// Saturday and Sunday.
pub(crate) fn time_bin(t: Timestamp, time_zone: &TimeZone) -> usize {
    let local = time_zone.to_datetime(t);
    let day_type = match local.weekday() {
        Weekday::Monday
        | Weekday::Tuesday
        | Weekday::Wednesday
        | Weekday::Thursday
        | Weekday::Friday => 0,
        Weekday::Saturday | Weekday::Sunday => 24,
    };
    usize::from(local.hour().unsigned_abs()) + day_type
}

/// Uniformisation: the largest expected number of events in one sub-step.
const MAX_STEP_EVENTS: f64 = 50.0;

/// Uniformisation: the Poisson mass that is left out of the sum.
const TAIL: f64 = 1e-12;

/// A square matrix, in row-major order.
#[derive(Clone, Debug)]
struct Matrix {
    size: usize,
    values: Vec<f64>,
}

impl Matrix {
    fn identity(size: usize) -> Self {
        Self::scaled_identity(size, 1.0)
    }

    fn scaled_identity(size: usize, scale: f64) -> Self {
        let mut values = vec![0.0; size * size];
        for i in 0..size {
            values[i * size + i] = scale;
        }
        Self { size, values }
    }

    fn multiply(&self, other: &Self) -> Self {
        let n = self.size;
        let mut values = vec![0.0; n * n];
        for i in 0..n {
            for k in 0..n {
                let a = self.values[i * n + k];
                if a != 0.0 {
                    for j in 0..n {
                        values[i * n + j] += a * other.values[k * n + j];
                    }
                }
            }
        }
        Self { size: n, values }
    }

    /// Returns the row vector `p` times the matrix.
    fn left_multiply(&self, p: &[f64]) -> Vec<f64> {
        let n = self.size;
        let mut result = vec![0.0; n];
        for (row, &weight) in self.values.chunks_exact(n).zip(p) {
            for (sum, &value) in result.iter_mut().zip(row) {
                *sum += weight * value;
            }
        }
        result
    }

    fn add_scaled(&mut self, other: &Self, scale: f64) {
        for (value, &add) in self.values.iter_mut().zip(&other.values) {
            *value += scale * add;
        }
    }

    fn power(&self, mut exponent: usize) -> Self {
        let mut result = Self::identity(self.size);
        let mut base = self.clone();
        while exponent > 0 {
            if exponent % 2 == 1 {
                result = result.multiply(&base);
            }
            exponent /= 2;
            if exponent > 0 {
                base = base.multiply(&base);
            }
        }
        result
    }
}

/// Returns the rates of a birth and of a death in each state.
fn rates(birth: f64, death: f64, states: usize, mode: Mode) -> (Vec<f64>, Vec<f64>) {
    let up = (0..states)
        .map(|n| if n + 1 < states { birth } else { 0.0 })
        .collect();
    #[expect(
        clippy::cast_precision_loss,
        reason = "the number of states is far below 2^52"
    )]
    let down = (0..states)
        .map(|n| match (n, mode) {
            (0, Mode::Pool | Mode::Bike) => 0.0,
            (_, Mode::Pool) => death,
            (_, Mode::Bike) => n as f64 * death,
        })
        .collect();
    (up, down)
}

/// Returns the transition matrix of the chain over `minutes`, by
/// uniformisation.
fn transition(birth: f64, death: f64, minutes: i64, states: usize, mode: Mode) -> Matrix {
    let (up, down) = rates(birth, death, states, mode);
    let uniform = up.iter().zip(&down).map(|(u, d)| u + d).fold(0.0, f64::max);
    if uniform == 0.0 || minutes == 0 {
        return Matrix::identity(states);
    }
    #[expect(clippy::cast_precision_loss, reason = "minutes are few")]
    let events = uniform * minutes as f64;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive number of sub-steps, far below 2^32"
    )]
    let steps = ((events / MAX_STEP_EVENTS).ceil() as usize).max(1);
    #[expect(clippy::cast_precision_loss, reason = "steps are few")]
    let x = events / steps as f64;

    let mut jump = Matrix::identity(states);
    for n in 0..states {
        jump.values[n * states + n] = 1.0 - (up[n] + down[n]) / uniform;
        if n + 1 < states {
            jump.values[n * states + n + 1] = up[n] / uniform;
        }
        if n > 0 {
            jump.values[n * states + n - 1] = down[n] / uniform;
        }
    }

    let mut weight = (-x).exp();
    let mut term = Matrix::identity(states);
    let mut result = Matrix::scaled_identity(states, weight);
    let mut mass = weight;
    let mut k = 0.0;
    while 1.0 - mass > TAIL && k < x + 20.0 * x.sqrt() + 50.0 {
        k += 1.0;
        term = term.multiply(&jump);
        weight *= x / k;
        result.add_scaled(&term, weight);
        mass += weight;
    }
    result.power(steps)
}

/// Returns the distribution of the count at the origin: `certain` plus a
/// binomial number of the `suspect` bikes, each present with probability
/// `share`. Counts above the largest state are put in the largest state.
fn initial(certain: usize, suspect: usize, share: f64, states: usize) -> Vec<f64> {
    let mut p = vec![0.0; states];
    let mut choose = 1.0;
    for k in 0..=suspect {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "counts of bikes are small"
        )]
        let probability = choose * share.powi(k as i32) * (1.0 - share).powi((suspect - k) as i32);
        p[(certain + k).min(states - 1)] += probability;
        #[expect(clippy::cast_precision_loss, reason = "counts of bikes are small")]
        let next = (suspect - k) as f64 / (k + 1) as f64;
        choose *= next;
    }
    p
}

/// Returns `t` rounded down to the hour, in UTC.
fn floor_hour(t: Timestamp) -> Timestamp {
    t.round(
        TimestampRound::new()
            .smallest(Unit::Hour)
            .mode(RoundMode::Floor),
    )
    .unwrap_or(t)
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators as gs;

    use super::Matrix;
    use super::Mode;
    use super::transition;

    fn row(matrix: &Matrix, i: usize) -> &[f64] {
        &matrix.values[i * matrix.size..(i + 1) * matrix.size]
    }

    #[test]
    fn the_pool_reaches_the_steady_state_of_a_single_server_queue() {
        // M/M/1: P(0) = 1 - birth / death.
        let p = transition(0.02, 0.05, 5_000, 80, Mode::Pool);

        assert!((row(&p, 0)[0] - 0.6).abs() < 1e-6, "{}", row(&p, 0)[0]);
    }

    #[test]
    fn a_hazard_per_bike_reaches_a_poisson_steady_state() {
        // M/M/infinity: the count is Poisson with mean birth / hazard.
        let p = transition(0.1, 0.02, 5_000, 60, Mode::Bike);

        assert!((row(&p, 0)[0] - (-5.0_f64).exp()).abs() < 1e-6);
    }

    #[test]
    fn births_alone_are_poisson() {
        let p = transition(0.05, 0.0, 60, 60, Mode::Pool);

        let mut expected = (-3.0_f64).exp();
        for k in 0..10 {
            assert!((row(&p, 2)[2 + k] - expected).abs() < 1e-12, "{k}");
            #[expect(clippy::cast_precision_loss, reason = "k is small")]
            let next = 3.0 / (k + 1) as f64;
            expected *= next;
        }
    }

    #[test]
    fn no_rates_give_the_identity() {
        let p = transition(0.0, 0.0, 60, 5, Mode::Bike);

        assert_eq!(p.values, Matrix::identity(5).values);
    }

    #[hegel::test(test_cases = 50)]
    fn each_row_is_a_distribution(tc: TestCase) {
        let birth = tc.draw(gs::floats::<f64>().min_value(0.0).max_value(1.0));
        let death = tc.draw(gs::floats::<f64>().min_value(0.0).max_value(1.0));
        let minutes = tc.draw(gs::integers::<i64>().min_value(0).max_value(60));
        let states = tc.draw(gs::integers::<usize>().min_value(2).max_value(60));
        let mode = if tc.draw(gs::booleans()) {
            Mode::Pool
        } else {
            Mode::Bike
        };

        let p = transition(birth, death, minutes, states, mode);

        for i in 0..states {
            let values = row(&p, i);
            let sum: f64 = values.iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "row {i} sums to {sum}");
            assert!(values.iter().all(|&v| v >= -1e-12), "row {i}: {values:?}");
        }
    }
}

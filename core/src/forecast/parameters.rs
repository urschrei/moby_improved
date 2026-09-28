use std::collections::BTreeMap;
use std::collections::HashMap;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::Deserialize;

use super::chain::BINS;
use super::chain::Chain;
use crate::gbfs::ZoneHash;

/// The format of the parameter file that this crate reads.
const FORMAT: u32 = 1;

/// The fitted forecast model, from the parameter file of `analysis/fit.py`.
///
/// The format is in `analysis/schema/forecast_parameters.schema.json` in the
/// `moby_analysis` repository. Bays are identified by zone hash (see
/// [`crate::gbfs::zone_hash`]).
#[derive(Clone, Debug)]
pub struct Parameters {
    /// The version of the model that made the parameters.
    pub model: String,
    /// The time of the fit.
    pub fitted_at: Timestamp,
    /// The data that the fit used.
    pub history: History,
    /// How bikes leave a reach.
    pub mode: Mode,
    /// The rules that the fit used to find rentable bikes in bays.
    pub rules: Rules,
    /// The probability that a suspect bike at the origin is rentable.
    pub suspect_share: f64,
    /// The smallest number of states of a chain.
    pub min_states: usize,
    /// The reach of each place, by name.
    pub places: BTreeMap<String, Place>,
    /// The scores of the model in the backtest, against climatology.
    pub backtest: Vec<Score>,
    arrival: Rates,
    departure: Rates,
    time_zone: TimeZone,
}

/// The data that a fit used.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct History {
    /// The first accepted poll.
    pub start: Timestamp,
    /// The last accepted poll.
    pub end: Timestamp,
    /// The number of accepted polls.
    pub polls: u64,
}

/// How bikes leave the reach of a place.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// At the departure rate of the reach while it has at least one bike.
    Pool,
    /// Each bike at the departure rate: n bikes leave at n times the rate.
    Bike,
}

/// The rules that a fit used to find rentable bikes in bays.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// The vehicle type whose rules give the parking bays.
    pub vehicle_type_id: String,
    /// The minimum range of a rentable bike, in metres.
    pub min_range_m: f64,
    /// A bike is in the nearest bay within this distance, in metres.
    pub bay_distance_m: f64,
    /// A longer step between two positions of a bike is a move, in metres.
    pub move_m: f64,
    /// The time for which a bike in a suspect stay is suspect, in minutes.
    pub stale_max_min: f64,
    /// The polls in sequence before a bike is in or out of a bay.
    pub hysteresis_polls: u32,
    /// The walking distance of the reach of a place, in metres.
    pub reach_m: f64,
    /// The estimated walking distance is the straight-line distance times
    /// this factor.
    pub detour_factor: f64,
}

/// The reach of a place.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Place {
    /// The zone hashes of the bays within the reach.
    pub bays: Vec<ZoneHash>,
}

/// The backtest score of the model for one horizon and group of targets.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Score {
    /// `all`, or `07:00-10:00` for target times from 07:00 to 10:00.
    pub targets: String,
    /// The horizon, for example `15 min` or `overnight`.
    pub horizon: String,
    /// The number of pairs of forecast and outcome.
    pub pairs: u64,
    /// The Brier score of the model.
    pub brier: f64,
    /// The Brier score of climatology on the same pairs.
    pub reference_brier: f64,
    /// 1 - `brier` / `reference_brier`.
    pub skill: Option<f64>,
    /// The 2.5% point of the skill in a bootstrap over days.
    pub skill_low: Option<f64>,
    /// The 97.5% point of the skill in a bootstrap over days.
    pub skill_high: Option<f64>,
}

impl Parameters {
    /// Parses a parameter file.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Parameters`] if the body is not a parameter
    /// file, [`crate::Error::UnknownFormat`] for a format that the crate does
    /// not know, [`crate::Error::ProfileLength`] if a profile does not have
    /// [`BINS`] values, [`crate::Error::OutOfRange`] for a value outside its
    /// range, [`crate::Error::UnsupportedTimeZone`] for a time zone other
    /// than `Europe/Dublin`, and [`crate::Error::TimeZoneDatabase`] if the
    /// time zone database does not have it.
    pub fn from_slice(body: &[u8]) -> Result<Self, crate::Error> {
        let version: Version = serde_json::from_slice(body).map_err(crate::Error::Parameters)?;
        if version.format != FORMAT {
            return Err(crate::Error::UnknownFormat(version.format));
        }
        let raw: RawParameters = serde_json::from_slice(body).map_err(crate::Error::Parameters)?;
        // Chain::advance steps at UTC hours, which are local hours only in
        // a time zone with whole-hour offsets.
        if raw.timezone != TIME_ZONE {
            return Err(crate::Error::UnsupportedTimeZone(raw.timezone));
        }
        let time_zone = TimeZone::get(TIME_ZONE).map_err(crate::Error::TimeZoneDatabase)?;
        if !(0.0..=1.0).contains(&raw.suspect_share) {
            return Err(out_of_range("suspect_share", raw.suspect_share));
        }
        if raw.min_states < 2 {
            #[expect(clippy::cast_precision_loss, reason = "the value is below 2 and exact")]
            let value = raw.min_states as f64;
            return Err(out_of_range("min_states", value));
        }
        Ok(Self {
            model: raw.model,
            fitted_at: raw.fitted_at,
            history: raw.history,
            mode: raw.mode,
            rules: raw.rules,
            suspect_share: raw.suspect_share,
            min_states: raw.min_states,
            places: raw.places,
            backtest: raw.backtest,
            arrival: raw.arrival.try_into()?,
            departure: raw.departure.try_into()?,
            time_zone,
        })
    }

    /// Returns the chain of a reach, from the zone hashes of its bays.
    ///
    /// The birth rate is the sum of the arrival rates of the bays. In
    /// [`Mode::Pool`], the death rate is the sum of their departure rates; in
    /// [`Mode::Bike`], it is the hazard of the bays together. A bay that is
    /// not in the parameters (a new zone) has the rates of all bays. A reach
    /// with no bays has no births and no deaths.
    #[must_use]
    pub fn reach<S: AsRef<str>>(&self, bays: &[S]) -> Chain {
        let arrival = self.arrival.sum_of_factors(bays);
        let departure = match self.mode {
            Mode::Pool => self.departure.sum_of_factors(bays),
            Mode::Bike => self.departure.pooled_factor(bays),
        };
        Chain::new(
            &self.arrival.profile.map(|rate| arrival * rate),
            &self.departure.profile.map(|rate| departure * rate),
            self.mode,
            self.suspect_share,
            self.min_states,
            self.time_zone.clone(),
        )
    }

    /// Returns the chain of a place in the parameters, if there is one.
    #[must_use]
    pub fn place(&self, name: &str) -> Option<Chain> {
        self.places.get(name).map(|place| self.reach(&place.bays))
    }
}

/// The only time zone of the parameters.
const TIME_ZONE: &str = "Europe/Dublin";

/// Rates of the form factor\[bay\] * profile\[bin\], per minute of exposure.
#[derive(Clone, Debug)]
struct Rates {
    profile: [f64; BINS],
    /// The prior of the factors is a gamma distribution with mean 1, and
    /// shape and rate `strength`.
    strength: f64,
    bays: HashMap<ZoneHash, Exposure>,
}

/// The events of a bay in the history, and the events that the profile
/// gives for its exposure.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
struct Exposure(f64, f64);

impl Rates {
    /// Returns the posterior factor of one bay: (events + strength) /
    /// (expected + strength). A bay that is not known has the factor 1.
    fn factor(&self, bay: &str) -> f64 {
        let Exposure(events, expected) = self.exposure(bay);
        (events + self.strength) / (expected + self.strength)
    }

    fn sum_of_factors<S: AsRef<str>>(&self, bays: &[S]) -> f64 {
        bays.iter().map(|bay| self.factor(bay.as_ref())).sum()
    }

    /// Returns the posterior factor of the bays together, or 0 if there are
    /// no bays.
    fn pooled_factor<S: AsRef<str>>(&self, bays: &[S]) -> f64 {
        if bays.is_empty() {
            return 0.0;
        }
        let (events, expected) = bays
            .iter()
            .map(|bay| self.exposure(bay.as_ref()))
            .fold((0.0, 0.0), |(e, x), Exposure(events, expected)| {
                (e + events, x + expected)
            });
        #[expect(
            clippy::cast_precision_loss,
            reason = "a reach has far fewer than 2^52 bays"
        )]
        let prior = bays.len() as f64 * self.strength;
        (events + prior) / (expected + prior)
    }

    fn exposure(&self, bay: &str) -> Exposure {
        self.bays.get(bay).copied().unwrap_or_default()
    }
}

impl TryFrom<RawRates> for Rates {
    type Error = crate::Error;

    fn try_from(raw: RawRates) -> Result<Self, Self::Error> {
        let length = raw.profile.len();
        let profile: [f64; BINS] = raw
            .profile
            .try_into()
            .map_err(|_| crate::Error::ProfileLength(length))?;
        if let Some(&rate) = profile
            .iter()
            .find(|rate| !rate.is_finite() || **rate < 0.0)
        {
            return Err(out_of_range("profile", rate));
        }
        if !(raw.strength.is_finite() && raw.strength > 0.0) {
            return Err(out_of_range("strength", raw.strength));
        }
        let invalid = raw
            .bays
            .values()
            .flat_map(|&Exposure(events, expected)| [events, expected])
            .find(|value| !value.is_finite() || *value < 0.0);
        if let Some(value) = invalid {
            return Err(out_of_range("bays", value));
        }
        Ok(Self {
            profile,
            strength: raw.strength,
            bays: raw.bays,
        })
    }
}

fn out_of_range(name: &'static str, value: f64) -> crate::Error {
    crate::Error::OutOfRange { name, value }
}

/// The format field alone, so that a file of another format is refused
/// before its other fields are read.
#[derive(Deserialize)]
struct Version {
    format: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawParameters {
    #[expect(dead_code, reason = "read by `Version`")]
    format: u32,
    model: String,
    fitted_at: Timestamp,
    history: History,
    timezone: String,
    mode: Mode,
    rules: Rules,
    suspect_share: f64,
    min_states: usize,
    arrival: RawRates,
    departure: RawRates,
    places: BTreeMap<String, Place>,
    backtest: Vec<Score>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRates {
    profile: Vec<f64>,
    strength: f64,
    bays: HashMap<ZoneHash, Exposure>,
}

use jiff::SignedDuration;
use jiff::Timestamp;
use serde::Deserialize;

use super::LEAD;
use crate::Error;

/// The times after the request at which the app shows a forecast.
pub const AHEAD: [SignedDuration; 3] = [
    SignedDuration::from_mins(30),
    SignedDuration::from_mins(60),
    SignedDuration::from_mins(120),
];

/// The horizon bins that the app shows. At gate G4 of the forecast plan
/// (2026-11-08), the model beats climatology in these bins and not at 6 h
/// or overnight.
pub const SHOWN_HORIZONS: [&str; 5] = ["5 min", "15 min", "30 min", "60 min", "2 h"];

/// The step of a rounded percentage.
const PERCENT_STEP: f64 = 5.0;

/// The lowest and the highest rounded percentage. The model is not certain,
/// so the app does not show 0% or 100%.
const PERCENT_RANGE: (u8, u8) = (5, 95);

/// Returns the target times to request at `now`, one for each of [`AHEAD`].
#[must_use]
pub fn request_targets(now: Timestamp) -> [Timestamp; 3] {
    AHEAD.map(|ahead| now + ahead - LEAD)
}

/// A forecast for a place, from `GET /v1/forecast` of `moby-collector` in
/// the `moby_analysis` repository.
///
/// Unknown fields are ignored, so that the service can add fields.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Response {
    /// The name of the place.
    pub place: String,
    /// The time of the request at the service.
    pub issued_at: Timestamp,
    /// The time of the last accepted poll: the origin of the forecast.
    pub origin: Timestamp,
    /// The rentable bikes in the reach at the origin that are not suspect.
    pub certain: u32,
    /// The bikes in the reach at the origin that can be at a stale position
    /// or in transit.
    pub suspect: u32,
    /// One forecast for each target, in the order of the request.
    pub targets: Vec<Target>,
    /// The model that made the forecast.
    pub model: Model,
}

/// The body of a refusal of the service for old parameters or an old poll,
/// with the count at the last accepted poll. The forecast is paused, and the
/// count is still correct.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Paused {
    /// Why the service gives no forecast.
    pub reason: PauseReason,
    /// The time of the last accepted poll.
    pub origin: Timestamp,
    /// The time of the fit of the parameters.
    pub fitted_at: Timestamp,
    /// The rentable bikes in the reach at the origin that are not suspect.
    pub certain: u32,
    /// The bikes in the reach at the origin that can be at a stale position
    /// or in transit.
    pub suspect: u32,
}

/// Why the service gives a count and no forecast.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PauseReason {
    /// The parameters are more than 36 h old: the nightly fit did not run.
    ParametersTooOld,
    /// The last accepted poll is more than 10 min old: the feed of MOBY
    /// stopped.
    FeedTooOld,
}

impl Paused {
    /// Parses the body of a refusal with status 503.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Forecast`] if the body does not have a reason and
    /// the count, for example from a version of the service before
    /// 2026-11-08, or for a failure of the service.
    pub fn from_slice(body: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(body).map_err(Error::Forecast)
    }
}

/// The forecast for one target time.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Target {
    /// The target time.
    pub at: Timestamp,
    /// The probability of at least one rentable bike in the reach.
    pub probability: f64,
    /// The horizon bin of the backtest for this target.
    pub horizon: String,
    /// The skill of the model against climatology in the backtest for this
    /// horizon bin. `None` if the backtest has no score.
    pub skill: Option<f64>,
}

/// The model that made a forecast.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Model {
    /// The version of the model.
    pub version: String,
    /// The time of the fit.
    pub fitted_at: Timestamp,
    /// How bikes leave a reach.
    pub mode: String,
}

impl Response {
    /// Parses the body of a forecast response.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Forecast`] if the body is not a forecast, and
    /// [`Error::OutOfRange`] if a probability is not in [0, 1].
    pub fn from_slice(body: &[u8]) -> Result<Self, Error> {
        let response: Self = serde_json::from_slice(body).map_err(Error::Forecast)?;
        if let Some(target) = response
            .targets
            .iter()
            .find(|target| !(0.0..=1.0).contains(&target.probability))
        {
            return Err(Error::OutOfRange {
                name: "probability",
                value: target.probability,
            });
        }
        Ok(response)
    }
}

impl Target {
    /// Returns `true` if the app shows this target: its horizon bin is one of
    /// [`SHOWN_HORIZONS`] and its skill is above 0.
    #[must_use]
    pub fn is_shown(&self) -> bool {
        SHOWN_HORIZONS.contains(&self.horizon.as_str())
            && self.skill.is_some_and(|skill| skill > 0.0)
    }

    /// Returns the probability as a percentage, rounded to the nearest 5%,
    /// from 5% to 95%.
    #[must_use]
    pub fn percent(&self) -> u8 {
        let rounded = (self.probability * 100.0 / PERCENT_STEP).round() * PERCENT_STEP;
        let (low, high) = PERCENT_RANGE;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the rounded value is a whole number in [0, 100]"
        )]
        let percent = rounded as u8;
        percent.clamp(low, high)
    }
}

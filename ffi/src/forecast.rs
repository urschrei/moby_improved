use moby_core::forecast::Aim;
use moby_core::forecast::Paused;
use moby_core::forecast::Response;

use crate::MobyError;
use crate::schedule::CommuteWindow;
use crate::schedule::schedule;
use crate::schedule::zoned;

/// When the app shows a forecast, from [`forecast_plan`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ForecastPlan {
    /// A commute window contains the time, or starts at most 2 h after it.
    Open {
        /// The window and the target.
        aim: ForecastAim,
    },
    /// No window contains the time or starts soon after it.
    Closed {
        /// The time at which the forecast opens, 2 h before the start of the
        /// next window, in milliseconds since the epoch. `None` if there
        /// are no windows.
        opens_ms: Option<i64>,
    },
}

/// The window of a forecast and its target, in milliseconds since the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ForecastAim {
    /// The start of the window.
    pub start_ms: i64,
    /// The end of the window.
    pub end_ms: i64,
    /// The time of the forecast that the app shows: the start of the
    /// window, or 15 min after the request in the window.
    pub at_ms: i64,
    /// The target to send to the service. It is `at_ms`, or up to 2 min
    /// before it if `at_ms` is about 2 h ahead.
    pub request_ms: i64,
}

/// The forecast for a saved place, as the app shows it.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct PlaceForecast {
    /// The name of the place at the service.
    pub place: String,
    /// The time of the request at the service, in milliseconds since the
    /// epoch.
    pub issued_ms: i64,
    /// The time of the last accepted poll, in milliseconds since the epoch.
    pub origin_ms: i64,
    /// The rentable bikes within 300 m walk at the origin.
    pub certain: u32,
    /// The other bikes within 300 m walk at the origin that a rider
    /// possibly returned at a stale position.
    pub suspect: u32,
    /// The probability of a rentable bike within 300 m walk at the target,
    /// in percent, rounded to 5. `None` if the backtest does not show skill
    /// at this horizon.
    pub percent: Option<u8>,
}

/// The count of a place when the service pauses the forecast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ForecastPause {
    /// Why the forecast is paused.
    pub reason: PauseReason,
    /// The time of the last accepted poll, in milliseconds since the epoch.
    pub origin_ms: i64,
    /// The time of the last fit, in milliseconds since the epoch.
    pub fitted_at_ms: i64,
    /// The rentable bikes within 300 m walk at the last poll.
    pub certain: u32,
    /// The other bikes within 300 m walk at the last poll that a rider
    /// possibly returned at a stale position.
    pub suspect: u32,
}

/// Why the service pauses the forecast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum PauseReason {
    /// The nightly fit did not run.
    ParametersTooOld,
    /// The feed of MOBY stopped.
    FeedTooOld,
}

/// Returns when the app shows a forecast at `now_ms`, for the commute
/// windows in `time_zone`.
///
/// # Errors
///
/// Returns an error if a window is not valid, or if the time or the time
/// zone is not valid.
#[uniffi::export]
pub fn forecast_plan(
    windows: &[CommuteWindow],
    now_ms: i64,
    time_zone: &str,
) -> Result<ForecastPlan, MobyError> {
    let now = zoned(now_ms, time_zone)?;
    Ok(match moby_core::forecast::aim(&schedule(windows)?, &now) {
        Aim::Open {
            start,
            end,
            at,
            request,
        } => ForecastPlan::Open {
            aim: ForecastAim {
                start_ms: start.as_millisecond(),
                end_ms: end.as_millisecond(),
                at_ms: at.as_millisecond(),
                request_ms: request.as_millisecond(),
            },
        },
        Aim::Closed { opens } => ForecastPlan::Closed {
            opens_ms: opens.map(jiff::Timestamp::as_millisecond),
        },
    })
}

/// Parses the body of a forecast response to a request with one target.
///
/// # Errors
///
/// Returns an error if the body is not a forecast, or if it does not have
/// exactly one target.
#[uniffi::export]
pub fn parse_forecast(body: &[u8]) -> Result<PlaceForecast, MobyError> {
    let response = Response::from_slice(body)?;
    let [target] = response.targets.as_slice() else {
        return Err(MobyError::Parse(format!(
            "the forecast has {} targets, not 1",
            response.targets.len()
        )));
    };
    Ok(PlaceForecast {
        percent: target.is_shown().then(|| target.percent()),
        place: response.place,
        issued_ms: response.issued_at.as_millisecond(),
        origin_ms: response.origin.as_millisecond(),
        certain: response.certain,
        suspect: response.suspect,
    })
}

/// Parses the body of a response with status 503 that pauses the forecast
/// and keeps the count.
///
/// # Errors
///
/// Returns an error if the body has no reason and count: the service
/// failed, or it is a version from before 2026-11-08.
#[uniffi::export]
pub fn parse_pause(body: &[u8]) -> Result<ForecastPause, MobyError> {
    let paused = Paused::from_slice(body)?;
    Ok(ForecastPause {
        reason: match paused.reason {
            moby_core::forecast::PauseReason::ParametersTooOld => PauseReason::ParametersTooOld,
            moby_core::forecast::PauseReason::FeedTooOld => PauseReason::FeedTooOld,
        },
        origin_ms: paused.origin.as_millisecond(),
        fitted_at_ms: paused.fitted_at.as_millisecond(),
        certain: paused.certain,
        suspect: paused.suspect,
    })
}

#[cfg(test)]
mod tests {
    use super::ForecastPlan;
    use super::PauseReason;
    use super::forecast_plan;
    use super::parse_forecast;
    use super::parse_pause;
    use crate::MobyError;
    use crate::schedule::CommuteWindow;

    fn body(targets: &[(&str, f64, &str)]) -> String {
        let targets: Vec<String> = targets
            .iter()
            .map(|(horizon, p, skill)| {
                format!(
                    r#"{{"at":"2026-11-08T10:00:00Z","probability":{p},"horizon":"{horizon}","skill":{skill}}}"#
                )
            })
            .collect();
        format!(
            r#"{{"place":"work","issued_at":"2026-11-08T09:30:00Z","origin":"2026-11-08T09:29:00Z","certain":2,"suspect":1,"targets":[{}],"model":{{"version":"v","fitted_at":"2026-11-08T02:30:00Z","mode":"bike"}}}}"#,
            targets.join(",")
        )
    }

    #[test]
    fn a_target_with_skill_has_a_percentage() {
        let forecast = parse_forecast(body(&[("30 min", 0.42, "0.5")]).as_bytes()).unwrap();

        assert_eq!((forecast.certain, forecast.suspect), (2, 1));
        assert_eq!(forecast.percent, Some(40));
        assert_eq!(forecast.issued_ms, 1_794_130_200_000);
    }

    #[test]
    fn a_target_without_skill_has_no_percentage() {
        let forecast = parse_forecast(body(&[("2 h", 0.78, "null")]).as_bytes()).unwrap();

        assert_eq!(forecast.percent, None);
    }

    #[test]
    fn a_forecast_without_exactly_one_target_is_refused() {
        let two = body(&[("30 min", 0.4, "0.5"), ("2 h", 0.7, "0.2")]);

        let Err(MobyError::Parse(message)) = parse_forecast(two.as_bytes()) else {
            panic!("expected a parse error");
        };
        assert!(message.contains("2 targets"), "{message}");
    }

    #[test]
    fn a_pause_has_its_reason_and_count() {
        let body = br#"{"error":"x","reason":"feed_too_old","origin":"2026-11-08T07:21:00Z","fitted_at":"2026-11-08T02:41:00Z","certain":3,"suspect":1}"#;

        let pause = parse_pause(body).unwrap();

        assert_eq!(pause.reason, PauseReason::FeedTooOld);
        assert_eq!((pause.certain, pause.suspect), (3, 1));
        assert!(parse_pause(br#"{"error":"x"}"#).is_err());
    }

    #[test]
    fn the_plan_opens_two_hours_before_a_window() {
        let windows = [CommuteWindow {
            weekdays: 0b001_1111,
            start_minute: 7 * 60 + 30,
            end_minute: 9 * 60 + 30,
        }];
        // Monday 9 November 2026, 05:00 in Dublin, which is UTC in winter.
        let now = 1_794_200_400_000;

        let plan = forecast_plan(&windows, now, "Europe/Dublin").unwrap();

        assert_eq!(
            plan,
            ForecastPlan::Closed {
                opens_ms: Some(now + 30 * 60_000)
            }
        );
        let ForecastPlan::Open { aim } =
            forecast_plan(&windows, now + 60 * 60_000, "Europe/Dublin").unwrap()
        else {
            panic!("expected an open plan");
        };
        assert_eq!(aim.at_ms, now + 150 * 60_000);
    }
}

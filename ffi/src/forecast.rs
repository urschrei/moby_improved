use jiff::Timestamp;
use moby_core::forecast::AHEAD;
use moby_core::forecast::Response;
use moby_core::forecast::request_targets;

use crate::MobyError;

/// The forecast for a saved place, as the app shows it.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct PlaceForecast {
    /// The name of the place at the service.
    pub place: String,
    /// The time of the last accepted poll, in milliseconds since the epoch.
    pub origin_ms: i64,
    /// The rentable bikes within 300 m walk at the origin.
    pub certain: u32,
    /// The other bikes within 300 m walk at the origin that a rider
    /// possibly returned at a stale position.
    pub suspect: u32,
    /// One target for each of [`AHEAD`], in order.
    pub targets: Vec<ForecastTarget>,
}

/// The forecast for one time after the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ForecastTarget {
    /// The time after the request, in minutes.
    pub ahead_min: i64,
    /// The target time, in milliseconds since the epoch.
    pub at_ms: i64,
    /// The probability of a rentable bike within 300 m walk, in percent,
    /// rounded to 5. `None` if the backtest does not show skill at this
    /// horizon.
    pub percent: Option<u8>,
}

/// Returns the target times of a forecast request at `now_ms`, in
/// milliseconds since the epoch. Send them in this order.
///
/// # Errors
///
/// Returns an error if `now_ms` is not a valid time.
#[uniffi::export]
pub fn forecast_targets_ms(now_ms: i64) -> Result<Vec<i64>, MobyError> {
    let now = Timestamp::from_millisecond(now_ms)
        .map_err(|error| MobyError::InvalidTime(error.to_string()))?;
    Ok(request_targets(now)
        .iter()
        .map(|at| at.as_millisecond())
        .collect())
}

/// Parses the body of a forecast response to a request with the targets of
/// [`forecast_targets_ms`].
///
/// # Errors
///
/// Returns an error if the body is not a forecast, or if it does not have
/// one target for each time of the request.
#[uniffi::export]
pub fn parse_forecast(body: &[u8]) -> Result<PlaceForecast, MobyError> {
    let response = Response::from_slice(body)?;
    if response.targets.len() != AHEAD.len() {
        return Err(MobyError::Parse(format!(
            "the forecast has {} targets, not {}",
            response.targets.len(),
            AHEAD.len()
        )));
    }
    let targets = AHEAD
        .iter()
        .zip(&response.targets)
        .map(|(ahead, target)| ForecastTarget {
            ahead_min: ahead.as_mins(),
            at_ms: target.at.as_millisecond(),
            percent: target.is_shown().then(|| target.percent()),
        })
        .collect();
    Ok(PlaceForecast {
        place: response.place,
        origin_ms: response.origin.as_millisecond(),
        certain: response.certain,
        suspect: response.suspect,
        targets,
    })
}

#[cfg(test)]
mod tests {
    use super::forecast_targets_ms;
    use super::parse_forecast;
    use crate::MobyError;

    fn body(skills: [&str; 3]) -> String {
        let targets: Vec<String> = [("30 min", 0.42), ("60 min", 0.61), ("2 h", 0.78)]
            .iter()
            .zip(skills)
            .map(|((horizon, p), skill)| {
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
    fn each_target_has_its_time_ahead_and_a_percentage_if_it_has_skill() {
        let forecast = parse_forecast(body(["0.5", "0.3", "null"]).as_bytes()).unwrap();

        assert_eq!((forecast.certain, forecast.suspect), (2, 1));
        let shown: Vec<_> = forecast
            .targets
            .iter()
            .map(|target| (target.ahead_min, target.percent))
            .collect();
        assert_eq!(shown, [(30, Some(40)), (60, Some(60)), (120, None)]);
    }

    #[test]
    fn a_forecast_without_a_target_for_each_time_is_refused() {
        let text = body(["0.5", "0.3", "0.2"]).replace(
            r#",{"at":"2026-11-08T10:00:00Z","probability":0.78,"horizon":"2 h","skill":0.2}"#,
            "",
        );

        let Err(MobyError::Parse(message)) = parse_forecast(text.as_bytes()) else {
            panic!("expected a parse error");
        };
        assert!(message.contains("2 targets"), "{message}");
    }

    #[test]
    fn the_request_has_three_targets_in_order() {
        let targets = forecast_targets_ms(1_762_594_200_000).unwrap();

        assert_eq!(targets.len(), 3);
        assert!(targets.windows(2).all(|pair| pair[0] < pair[1]));
    }
}

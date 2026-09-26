use jiff::Timestamp;
use jiff::Zoned;
use jiff::civil::Time;
use moby_core::Schedule;
use moby_core::Weekdays;

use crate::MobyError;

/// A daily commute window in local time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, uniffi::Record)]
pub struct CommuteWindow {
    /// The days of the window. Bit 0 is Monday and bit 6 is Sunday.
    pub weekdays: u8,
    /// The start of the window, in minutes after midnight.
    pub start_minute: u16,
    /// The end of the window (exclusive), in minutes after midnight.
    pub end_minute: u16,
}

/// Returns `true` if `now_ms` is in one of the windows, in `time_zone`.
///
/// # Errors
///
/// Returns an error if a window is not valid or the time zone is not known.
#[uniffi::export]
pub fn schedule_is_active(
    windows: &[CommuteWindow],
    now_ms: i64,
    time_zone: &str,
) -> Result<bool, MobyError> {
    let now = zoned(now_ms, time_zone)?;
    Ok(schedule(windows)?.is_active(&now))
}

/// Returns the first time after `now_ms` at which the result of
/// [`schedule_is_active`] can change, in milliseconds since the epoch.
///
/// # Errors
///
/// Returns an error if a window is not valid or the time zone is not known.
#[uniffi::export]
pub fn schedule_next_boundary_ms(
    windows: &[CommuteWindow],
    now_ms: i64,
    time_zone: &str,
) -> Result<Option<i64>, MobyError> {
    let now = zoned(now_ms, time_zone)?;
    Ok(schedule(windows)?
        .next_boundary(&now)
        .map(|boundary| boundary.timestamp().as_millisecond()))
}

fn schedule(windows: &[CommuteWindow]) -> Result<Schedule, MobyError> {
    windows
        .iter()
        .map(|window| {
            moby_core::CommuteWindow::new(
                Weekdays::from_bits(window.weekdays),
                time(window.start_minute)?,
                time(window.end_minute)?,
            )
            .ok_or_else(|| MobyError::InvalidWindow(format!("{window:?}")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Schedule::new)
}

fn time(minute: u16) -> Result<Time, MobyError> {
    let hour =
        i8::try_from(minute / 60).map_err(|_| MobyError::InvalidWindow(minute.to_string()))?;
    let minute =
        i8::try_from(minute % 60).map_err(|_| MobyError::InvalidWindow(minute.to_string()))?;
    Time::new(hour, minute, 0, 0).map_err(|error| MobyError::InvalidWindow(error.to_string()))
}

fn zoned(now_ms: i64, time_zone: &str) -> Result<Zoned, MobyError> {
    Timestamp::from_millisecond(now_ms)
        .and_then(|now| now.in_tz(time_zone))
        .map_err(|error| MobyError::InvalidTime(error.to_string()))
}

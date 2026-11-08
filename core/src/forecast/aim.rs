use jiff::SignedDuration;
use jiff::Timestamp;
use jiff::Zoned;

use crate::Schedule;

/// The longest time after the request for which the app shows a forecast.
/// At gate G4 of the forecast plan (2026-11-08), the model beats
/// climatology up to 2 h and not at 6 h or overnight.
pub const MAX_AHEAD: SignedDuration = SignedDuration::from_hours(2);

/// In a commute window, the target is this time after the request.
pub const IN_WINDOW_AHEAD: SignedDuration = SignedDuration::from_mins(15);

/// The app asks for a target at most this much before [`MAX_AHEAD`].
///
/// The service puts a target in a horizon bin by its time after the last
/// accepted poll, which is up to about a minute before the request. Without
/// the lead, a target 2 h after the request is in the 6 h bin.
pub const LEAD: SignedDuration = SignedDuration::from_mins(2);

/// When the app shows a forecast, and for which time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Aim {
    /// A commute window contains the request, or starts at most
    /// [`MAX_AHEAD`] after it.
    Open {
        /// The start of the window.
        start: Timestamp,
        /// The end of the window.
        end: Timestamp,
        /// The time of the forecast that the app shows: the start of the
        /// window, or [`IN_WINDOW_AHEAD`] after the request in the window.
        at: Timestamp,
        /// The target to request: `at`, but at most [`MAX_AHEAD`] less
        /// [`LEAD`] after the request.
        request: Timestamp,
    },
    /// No window contains the request or starts soon after it.
    Closed {
        /// The time at which the forecast opens, [`MAX_AHEAD`] before the
        /// start of the next window. `None` if the schedule has no windows.
        opens: Option<Timestamp>,
    },
}

/// Returns the aim of a forecast requested at `now`, for the commute windows
/// of `schedule`.
#[must_use]
pub fn aim(schedule: &Schedule, now: &Zoned) -> Aim {
    let request_time = now.timestamp();
    let Some(window) = schedule.current_or_next(now, MAX_AHEAD) else {
        return Aim::Closed {
            opens: schedule
                .next_start(now)
                .map(|start| start.timestamp() - MAX_AHEAD),
        };
    };
    let start = window.start.timestamp();
    let at = if start > request_time {
        start
    } else {
        request_time + IN_WINDOW_AHEAD
    };
    Aim::Open {
        start,
        end: window.end.timestamp(),
        at,
        request: at.min(request_time + MAX_AHEAD - LEAD),
    }
}

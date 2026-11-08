//! Tests of the aim of a forecast: when the app shows one, and for which
//! time, in Dublin time.

use hegel::TestCase;
use hegel::generators as gs;
use jiff::SignedDuration;
use jiff::Timestamp;
use jiff::Zoned;
use jiff::civil::time;
use moby_core::CommuteWindow;
use moby_core::Schedule;
use moby_core::Weekdays;
use moby_core::forecast::Aim;
use moby_core::forecast::IN_WINDOW_AHEAD;
use moby_core::forecast::LEAD;
use moby_core::forecast::MAX_AHEAD;
use moby_core::forecast::aim;

fn dublin(text: &str) -> Zoned {
    format!("{text}[Europe/Dublin]").parse().unwrap()
}

fn instant(text: &str) -> Timestamp {
    dublin(text).timestamp()
}

/// Windows from 07:30 to 09:30 and from 16:30 to 18:30 on workdays.
fn workdays() -> Schedule {
    Schedule::new(vec![
        CommuteWindow::new(Weekdays::WORKDAYS, time(7, 30, 0, 0), time(9, 30, 0, 0)).unwrap(),
        CommuteWindow::new(Weekdays::WORKDAYS, time(16, 30, 0, 0), time(18, 30, 0, 0)).unwrap(),
    ])
}

#[test]
fn before_a_window_the_aim_is_its_start() {
    let aim = aim(&workdays(), &dublin("2026-11-09T06:45:00"));

    assert_eq!(
        aim,
        Aim::Open {
            start: instant("2026-11-09T07:30:00"),
            end: instant("2026-11-09T09:30:00"),
            at: instant("2026-11-09T07:30:00"),
            request: instant("2026-11-09T07:30:00"),
        }
    );
}

#[test]
fn in_a_window_the_aim_is_a_little_ahead() {
    let Aim::Open { at, request, .. } = aim(&workdays(), &dublin("2026-11-09T08:10:00")) else {
        panic!("expected an open forecast");
    };

    assert_eq!(at, instant("2026-11-09T08:25:00"));
    assert_eq!(request, at);
}

#[test]
fn two_hours_before_a_window_the_request_is_early() {
    let Aim::Open { at, request, .. } = aim(&workdays(), &dublin("2026-11-09T05:30:00")) else {
        panic!("expected an open forecast");
    };

    assert_eq!(at, instant("2026-11-09T07:30:00"));
    assert_eq!(request, instant("2026-11-09T07:28:00"));
}

#[test]
fn far_from_a_window_the_forecast_opens_two_hours_before_it() {
    assert_eq!(
        aim(&workdays(), &dublin("2026-11-09T05:29:00")),
        Aim::Closed {
            opens: Some(instant("2026-11-09T05:30:00"))
        }
    );
    // Saturday: the next window is on Monday.
    assert_eq!(
        aim(&workdays(), &dublin("2026-11-14T08:00:00")),
        Aim::Closed {
            opens: Some(instant("2026-11-16T05:30:00"))
        }
    );
    assert_eq!(
        aim(&Schedule::default(), &dublin("2026-11-09T08:00:00")),
        Aim::Closed { opens: None }
    );
}

#[hegel::test(test_cases = 200)]
fn an_open_aim_is_requested_ahead_and_within_two_hours(tc: TestCase) {
    let now = (instant("2026-01-01T00:00:00")
        + SignedDuration::from_mins(
            tc.draw(gs::integers::<i64>().min_value(0).max_value(525_600)),
        ))
    .in_tz("Europe/Dublin")
    .unwrap();

    match aim(&workdays(), &now) {
        Aim::Open {
            start,
            end,
            at,
            request,
        } => {
            let now = now.timestamp();
            assert!(start < end);
            assert!(now < end, "{now} {end}");
            assert!(at == start || at == now + IN_WINDOW_AHEAD);
            assert!(request > now);
            assert!(request <= at);
            assert!(request <= now + MAX_AHEAD - LEAD);
        }
        Aim::Closed { opens } => {
            let opens = opens.unwrap();
            assert!(opens > now.timestamp(), "{now} {opens}");
        }
    }
}

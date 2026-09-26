//! Tests for commute windows, in Dublin time.

use hegel::TestCase;
use hegel::generators as gs;
use jiff::Timestamp;
use jiff::Zoned;
use jiff::civil::Time;
use jiff::civil::date;
use jiff::civil::time;
use moby_core::CommuteWindow;
use moby_core::Schedule;
use moby_core::Weekdays;

const TZ: &str = "Europe/Dublin";

fn dublin(text: &str) -> Zoned {
    format!("{text}[{TZ}]").parse().unwrap()
}

fn workday_schedule() -> Schedule {
    Schedule::new(vec![
        CommuteWindow::new(Weekdays::WORKDAYS, time(7, 30, 0, 0), time(9, 30, 0, 0)).unwrap(),
        CommuteWindow::new(Weekdays::WORKDAYS, time(16, 30, 0, 0), time(18, 30, 0, 0)).unwrap(),
    ])
}

fn minute(minutes: i16) -> Time {
    time(
        i8::try_from(minutes / 60).unwrap(),
        i8::try_from(minutes % 60).unwrap(),
        0,
        0,
    )
}

/// Draws a window. Half of the windows are in the early morning, where the
/// clocks change.
fn draw_window(tc: &TestCase) -> CommuteWindow {
    let latest = if tc.draw(gs::booleans()) { 240 } else { 1_439 };
    let start = tc.draw(gs::integers::<i16>().min_value(0).max_value(latest - 1));
    let end = tc.draw(gs::integers::<i16>().min_value(start + 1).max_value(latest));
    let weekdays = Weekdays::from_bits(tc.draw(gs::integers::<u8>()));
    CommuteWindow::new(weekdays, minute(start), minute(end)).unwrap()
}

fn draw_schedule(tc: &TestCase) -> Schedule {
    let count = tc.draw(gs::integers::<usize>().max_value(4));
    Schedule::new((0..count).map(|_| draw_window(tc)).collect())
}

/// Draws an instant in 2026, often near the clock changes on 29 March and
/// 25 October.
fn draw_instant(tc: &TestCase) -> Zoned {
    let second = if tc.draw(gs::booleans()) {
        let transition = tc.draw(gs::sampled_from(vec![
            "2026-03-29T01:00:00Z"
                .parse::<Timestamp>()
                .unwrap()
                .as_second(),
            "2026-10-25T01:00:00Z"
                .parse::<Timestamp>()
                .unwrap()
                .as_second(),
        ]));
        transition + tc.draw(gs::integers::<i64>().min_value(-7_200).max_value(7_200))
    } else {
        tc.draw(
            gs::integers::<i64>()
                .min_value(1_767_225_600)
                .max_value(1_798_761_600),
        )
    };
    Timestamp::from_second(second).unwrap().in_tz(TZ).unwrap()
}

#[test]
fn workday_morning_is_active() {
    // 2026-09-28 is a Monday.
    assert!(workday_schedule().is_active(&dublin("2026-09-28T08:00:00+01:00")));
}

#[test]
fn saturday_morning_is_not_active() {
    assert!(!workday_schedule().is_active(&dublin("2026-09-26T08:00:00+01:00")));
}

#[test]
fn window_end_is_exclusive() {
    let schedule = workday_schedule();

    assert!(schedule.is_active(&dublin("2026-09-28T07:30:00+01:00")));
    assert!(!schedule.is_active(&dublin("2026-09-28T09:30:00+01:00")));
}

#[test]
fn friday_evening_boundary_is_monday_morning() {
    let boundary = workday_schedule()
        .next_boundary(&dublin("2026-10-02T19:00:00+01:00"))
        .unwrap();

    assert_eq!(boundary, dublin("2026-10-05T07:30:00+01:00"));
}

#[test]
fn boundary_can_be_a_clock_change() {
    // The clocks go back at 02:00 on Sunday 25 October 2026.
    let boundary = workday_schedule()
        .next_boundary(&dublin("2026-10-24T19:00:00+01:00"))
        .unwrap();

    assert_eq!(boundary, dublin("2026-10-25T01:00:00+00:00"));
}

#[test]
fn window_must_start_before_it_ends() {
    assert_eq!(
        CommuteWindow::new(Weekdays::WORKDAYS, time(9, 0, 0, 0), time(9, 0, 0, 0)),
        None
    );
}

#[test]
fn schedule_without_days_has_no_boundary() {
    let schedule = Schedule::new(vec![
        CommuteWindow::new(Weekdays::from_bits(0), time(7, 0, 0, 0), time(9, 0, 0, 0)).unwrap(),
    ]);

    assert_eq!(
        schedule.next_boundary(&dublin("2026-09-28T08:00:00+01:00")),
        None
    );
}

#[test]
fn weekday_bits_start_on_monday() {
    let monday = Weekdays::from_bits(1);

    assert!(monday.contains(date(2026, 9, 28).weekday()));
    assert!(!monday.contains(date(2026, 9, 27).weekday()));
    assert_eq!(Weekdays::from_bits(0xff).bits(), 0x7f);
}

#[hegel::test]
fn active_means_the_local_time_is_in_the_window(tc: TestCase) {
    let window = draw_window(&tc);
    let now = draw_instant(&tc);

    let expected = window.weekdays().contains(now.weekday())
        && window.start() <= now.time()
        && now.time() < window.end();
    assert_eq!(Schedule::new(vec![window]).is_active(&now), expected);
}

#[hegel::test]
fn next_boundary_is_after_now_and_within_eight_days(tc: TestCase) {
    let schedule = draw_schedule(&tc);
    let now = draw_instant(&tc);

    if let Some(boundary) = schedule.next_boundary(&now) {
        assert!(boundary > now);
        assert!(boundary.timestamp() <= now.timestamp() + jiff::SignedDuration::from_hours(8 * 24));
    }
}

#[hegel::test]
fn activity_does_not_change_before_the_next_boundary(tc: TestCase) {
    let schedule = draw_schedule(&tc);
    let now = draw_instant(&tc);
    let Some(boundary) = schedule.next_boundary(&now) else {
        return;
    };
    let span_s = boundary.timestamp().as_second() - now.timestamp().as_second();
    let offset_s = tc.draw(gs::integers::<i64>().min_value(0).max_value(span_s - 1));
    let later = Timestamp::from_second(now.timestamp().as_second() + offset_s)
        .unwrap()
        .in_tz(TZ)
        .unwrap();

    assert_eq!(
        schedule.is_active(&later),
        schedule.is_active(&now),
        "{now} {later} {boundary}"
    );
}

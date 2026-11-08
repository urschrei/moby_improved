use jiff::SignedDuration;
use jiff::Zoned;
use jiff::civil::Date;
use jiff::civil::Time;
use jiff::civil::Weekday;
use jiff::tz::AmbiguousOffset;
use jiff::tz::TimeZone;

/// The times of the week when the rider commutes.
///
/// The app refreshes more often in a commute window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schedule {
    windows: Vec<CommuteWindow>,
}

/// A daily period on some days of the week, in local (wall clock) time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommuteWindow {
    weekdays: Weekdays,
    start: Time,
    end: Time,
}

/// A commute window on one date, as instants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence {
    /// The start of the window.
    pub start: Zoned,
    /// The end of the window.
    pub end: Zoned,
}

/// A set of days of the week.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Weekdays(u8);

impl Schedule {
    /// Makes a schedule from its windows.
    #[must_use]
    pub fn new(windows: Vec<CommuteWindow>) -> Self {
        Self { windows }
    }

    /// Returns `true` if `now` is in a commute window.
    #[must_use]
    pub fn is_active(&self, now: &Zoned) -> bool {
        self.windows.iter().any(|window| window.contains(now))
    }

    /// Returns the first instant after `now` at which [`Schedule::is_active`]
    /// can change, or `None` if the schedule has no windows on any day.
    ///
    /// The result is the first start or end of a window, or the first
    /// time-zone transition, after `now`. At a transition the wall clock
    /// jumps, so a window can start or end there, but the value of
    /// `is_active` does not always change.
    #[must_use]
    pub fn next_boundary(&self, now: &Zoned) -> Option<Zoned> {
        let tz = now.time_zone();
        let mut boundaries: Vec<Zoned> = Vec::new();
        // A window recurs within 7 days, so 8 days include the next start and
        // the next end of every window.
        let mut date = now.date();
        for _ in 0..8 {
            for window in &self.windows {
                if window.weekdays.contains(date.weekday()) {
                    for time in [window.start, window.end] {
                        // A wall clock time that occurs twice gives two
                        // boundaries. A time in a gap gives none, because the
                        // transition at the gap is a boundary.
                        let ambiguous = tz.to_ambiguous_zoned(date.to_datetime(time));
                        match ambiguous.offset() {
                            AmbiguousOffset::Unambiguous { .. } | AmbiguousOffset::Fold { .. } => {
                                boundaries.extend(ambiguous.clone().earlier());
                                boundaries.extend(ambiguous.later());
                            }
                            AmbiguousOffset::Gap { .. } => {}
                        }
                    }
                }
            }
            date = date.tomorrow().ok()?;
        }
        let next_window = boundaries
            .into_iter()
            .filter(|boundary| boundary > now)
            .min()?;

        let next_transition = tz
            .following(now.timestamp())
            .next()
            .map(|transition| transition.timestamp().to_zoned(tz.clone()));
        match next_transition {
            Some(transition) if transition < next_window => Some(transition),
            Some(_) | None => Some(next_window),
        }
    }
}

impl Schedule {
    /// Returns the window that contains `now`, or else the next window after
    /// `now` if it starts at most `within` after `now`.
    ///
    /// A start or end in a gap of the time zone is the instant after the gap.
    /// A window that contains `now` starts at `now` at the latest, also if
    /// its start is in a gap.
    #[must_use]
    pub fn current_or_next(&self, now: &Zoned, within: SignedDuration) -> Option<Occurrence> {
        if let Some(window) = self.windows.iter().find(|window| window.contains(now)) {
            return window
                .on(now.date(), now.time_zone())
                .map(|occurrence| Occurrence {
                    start: occurrence.start.min(now.clone()),
                    end: occurrence.end,
                });
        }
        let limit = now.timestamp().checked_add(within).ok()?;
        self.upcoming(now)
            .into_iter()
            .next()
            .filter(|occurrence| occurrence.start.timestamp() <= limit)
    }

    /// Returns the first start of a window after `now`, or `None` if the
    /// schedule has no windows on any day.
    #[must_use]
    pub fn next_start(&self, now: &Zoned) -> Option<Zoned> {
        self.upcoming(now)
            .into_iter()
            .next()
            .map(|occurrence| occurrence.start)
    }

    /// Returns the windows that start after `now`, in the next 8 days, in
    /// the order of their starts.
    fn upcoming(&self, now: &Zoned) -> Vec<Occurrence> {
        let mut occurrences = Vec::new();
        let mut date = now.date();
        for _ in 0..8 {
            occurrences.extend(
                self.windows
                    .iter()
                    .filter(|window| window.weekdays.contains(date.weekday()))
                    .filter_map(|window| window.on(date, now.time_zone()))
                    .filter(|occurrence| occurrence.start > *now),
            );
            let Ok(tomorrow) = date.tomorrow() else { break };
            date = tomorrow;
        }
        occurrences.sort_by(|a, b| a.start.cmp(&b.start));
        occurrences
    }
}

impl CommuteWindow {
    /// Makes a window from `start` (inclusive) to `end` (exclusive) on the
    /// given days.
    ///
    /// Returns `None` if `start` is not before `end`. A window cannot cross
    /// midnight.
    #[must_use]
    pub fn new(weekdays: Weekdays, start: Time, end: Time) -> Option<Self> {
        (start < end).then_some(Self {
            weekdays,
            start,
            end,
        })
    }

    /// Returns `true` if the wall clock time of `now` is in the window.
    #[must_use]
    pub fn contains(&self, now: &Zoned) -> bool {
        self.weekdays.contains(now.weekday()) && (self.start..self.end).contains(&now.time())
    }

    /// Returns the window on `date` in `tz`, whatever its days. Returns
    /// `None` if the date is at the end of the range of `jiff`.
    fn on(&self, date: Date, tz: &TimeZone) -> Option<Occurrence> {
        Some(Occurrence {
            start: date.to_datetime(self.start).to_zoned(tz.clone()).ok()?,
            end: date.to_datetime(self.end).to_zoned(tz.clone()).ok()?,
        })
    }

    /// Returns the days of the window.
    #[must_use]
    pub fn weekdays(&self) -> Weekdays {
        self.weekdays
    }

    /// Returns the start of the window.
    #[must_use]
    pub fn start(&self) -> Time {
        self.start
    }

    /// Returns the end of the window.
    #[must_use]
    pub fn end(&self) -> Time {
        self.end
    }
}

impl Weekdays {
    /// Monday to Friday.
    pub const WORKDAYS: Self = Self(0b001_1111);

    /// Makes a set from a bit mask. Bit 0 is Monday and bit 6 is Sunday.
    /// Other bits are ignored.
    #[must_use]
    pub fn from_bits(bits: u8) -> Self {
        Self(bits & 0b111_1111)
    }

    /// Returns the bit mask. Bit 0 is Monday and bit 6 is Sunday.
    #[must_use]
    pub fn bits(self) -> u8 {
        self.0
    }

    /// Returns `true` if the set contains `weekday`.
    #[must_use]
    pub fn contains(self, weekday: Weekday) -> bool {
        self.0 & (1 << weekday.to_monday_zero_offset()) != 0
    }
}

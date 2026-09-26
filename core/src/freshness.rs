use jiff::SignedDuration;
use jiff::Timestamp;

/// The age of a feed, and whether it is too old to trust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Freshness {
    /// The time since the publisher last updated the feed. It is zero if the
    /// feed timestamp is in the future, for example because of clock skew.
    pub age: SignedDuration,
    /// `true` if `age` is more than the stale threshold.
    pub is_stale: bool,
}

impl Freshness {
    /// Measures the age of a feed at the time `now`.
    #[must_use]
    pub fn measure(last_updated: Timestamp, now: Timestamp, stale_after: SignedDuration) -> Self {
        let age = now.duration_since(last_updated).max(SignedDuration::ZERO);
        Self {
            age,
            is_stale: age > stale_after,
        }
    }
}

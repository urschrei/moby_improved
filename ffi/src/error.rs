/// Errors that cross the FFI boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum MobyError {
    /// A feed body or a forecast parameter file did not parse, or is not
    /// valid.
    #[error("{0}")]
    Parse(String),
    /// The manifest does not list a feed that the app needs.
    #[error("{0}")]
    MissingFeed(String),
    /// A commute window does not start before it ends, or is not a time of
    /// day.
    #[error("invalid commute window: {0}")]
    InvalidWindow(String),
    /// A time or a time zone is not valid.
    #[error("invalid time: {0}")]
    InvalidTime(String),
}

impl From<moby_core::Error> for MobyError {
    fn from(error: moby_core::Error) -> Self {
        match error {
            moby_core::Error::Parse { .. }
            | moby_core::Error::Parameters(_)
            | moby_core::Error::UnknownFormat(_)
            | moby_core::Error::ProfileLength(_)
            | moby_core::Error::OutOfRange { .. }
            | moby_core::Error::UnsupportedTimeZone(_)
            | moby_core::Error::TimeZoneDatabase(_) => Self::Parse(error.to_string()),
            moby_core::Error::MissingFeed(_) => Self::MissingFeed(error.to_string()),
        }
    }
}

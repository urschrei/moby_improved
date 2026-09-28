/// Errors from parsing a feed or the parameters of the forecast.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The response body is not valid JSON for the expected feed.
    #[error("cannot parse {feed} feed: {source}")]
    Parse {
        /// The feed that failed to parse.
        feed: &'static str,
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// The manifest does not list a feed that the client needs.
    #[error("manifest does not list the {0} feed")]
    MissingFeed(&'static str),
    /// The body is not a forecast parameter file.
    #[error("cannot parse forecast parameters: {0}")]
    Parameters(#[source] serde_json::Error),
    /// The parameter file has a format that the crate does not know.
    #[error("unknown format of forecast parameters: {0}")]
    UnknownFormat(u32),
    /// A profile of the parameters does not have one value for each time
    /// bin.
    #[error("a profile of the forecast parameters has {0} values, not 48")]
    ProfileLength(usize),
    /// A value of the parameters is outside its range.
    #[error("forecast parameter {name} is out of range: {value}")]
    OutOfRange {
        /// The name of the value.
        name: &'static str,
        /// The value.
        value: f64,
    },
    /// The time zone of the parameters is not `Europe/Dublin`.
    #[error("unsupported time zone of forecast parameters: {0}")]
    UnsupportedTimeZone(String),
    /// The time zone database does not have `Europe/Dublin`.
    #[error("cannot find Europe/Dublin in the time zone database: {0}")]
    TimeZoneDatabase(#[source] jiff::Error),
}

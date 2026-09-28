/// Errors from parsing a feed.
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
    /// The forecast parameters cannot be used: for example, their format is
    /// not known.
    #[error("invalid forecast parameters: {0}")]
    InvalidParameters(String),
}

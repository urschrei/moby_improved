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
}

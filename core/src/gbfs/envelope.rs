use jiff::Timestamp;
use serde::Deserialize;

/// The common wrapper around every GBFS 3.0 feed.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Envelope<T> {
    /// The time at which the publisher last updated the data.
    pub last_updated: Timestamp,
    /// The number of seconds before the publisher will update the data.
    pub ttl: u32,
    /// The GBFS version of the feed.
    pub version: String,
    /// The feed payload.
    pub data: T,
}

impl<T: serde::de::DeserializeOwned> Envelope<T> {
    /// Parses a feed from a response body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Parse`] if the body does not match the feed.
    pub fn from_slice(feed: &'static str, body: &[u8]) -> Result<Self, crate::Error> {
        serde_json::from_slice(body).map_err(|source| crate::Error::Parse { feed, source })
    }
}

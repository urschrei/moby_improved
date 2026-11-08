//! The availability forecast: the probability of at least one rentable bike
//! within walking distance of a place at a target time.
//!
//! The model is fitted by `analysis/fit.py` in the `moby_analysis`
//! repository, which writes the parameter file that [`Parameters`] reads.
//! The bikes in the reach of a place are a birth-death chain; see
//! [`Chain`]. The collector serves forecasts from the parameters; see
//! [`Response`].

mod aim;
mod chain;
mod parameters;
mod response;

pub use aim::Aim;
pub use aim::IN_WINDOW_AHEAD;
pub use aim::LEAD;
pub use aim::MAX_AHEAD;
pub use aim::aim;
pub use chain::BINS;
pub use chain::Chain;
pub use chain::MAX_CACHED;
pub use parameters::History;
pub use parameters::Mode;
pub use parameters::Parameters;
pub use parameters::Place;
pub use parameters::Rules;
pub use parameters::Score;
pub use response::Model;
pub use response::PauseReason;
pub use response::Paused;
pub use response::Response;
pub use response::SHOWN_HORIZONS;
pub use response::Target;

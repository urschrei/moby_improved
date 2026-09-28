//! The availability forecast: the probability of at least one rentable bike
//! within walking distance of a place at a target time.
//!
//! The model is fitted by `analysis/fit.py` in the `moby_analysis`
//! repository, which writes the parameter file that [`Parameters`] reads.
//! The bikes in the reach of a place are a birth-death chain; see
//! [`Chain`].

mod chain;
mod parameters;

pub use chain::BINS;
pub use chain::Chain;
pub use chain::states_for;
pub use chain::time_bin;
pub use parameters::History;
pub use parameters::Mode;
pub use parameters::Parameters;
pub use parameters::Place;
pub use parameters::Rules;
pub use parameters::Score;

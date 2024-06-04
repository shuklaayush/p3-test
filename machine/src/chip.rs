use core::fmt::{Debug, Display};

#[cfg(feature = "air-logger")]
use p3_air_util::AirLogger;

#[cfg(not(feature = "air-logger"))]
pub trait Chip: Clone + Debug + Display {}

#[cfg(feature = "air-logger")]
pub trait Chip: Clone + Debug + Display + AirLogger {}

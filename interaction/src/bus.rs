use core::fmt::Display;

pub trait Bus: Sized + From<usize> + Display {}

use p3_matrix::{dense::RowMajorMatrixView, stack::VerticalPair};

mod debug;
mod prover;
mod symbolic;
mod verifier;

pub use debug::*;
pub use prover::*;
pub use symbolic::*;
pub use verifier::*;

pub type ViewPair<'a, T> = VerticalPair<RowMajorMatrixView<'a, T>, RowMajorMatrixView<'a, T>>;

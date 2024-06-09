pub mod air;
pub mod rap;

use alloc::collections::BTreeSet;
use p3_matrix::{dense::RowMajorMatrixView, stack::VerticalPair};

pub type ViewPair<'a, T> = VerticalPair<RowMajorMatrixView<'a, T>, RowMajorMatrixView<'a, T>>;

#[derive(Default, Clone)]
pub struct EntriesLog<T: Copy + Ord> {
    pub failing: BTreeSet<T>,
    pub constrained: BTreeSet<T>,
}

impl<T: Copy + Ord> EntriesLog<T> {
    pub fn extend(&mut self, other: &Self) {
        self.failing.extend(&other.failing);
        self.constrained.extend(&other.constrained);
    }
}

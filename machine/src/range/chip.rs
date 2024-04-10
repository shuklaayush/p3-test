extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::mem::transmute;
use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;

use super::{
    columns::{RangeCols, NUM_RANGE_COLS, RANGE_COL_MAP, RANGE_PREPROCESSED_COL_MAP},
    RangeCheckerChip,
};
use crate::{chip::Chip, interaction::Interaction, machine::MachineBus};

impl<F: PrimeField64, const MAX: u32> Chip<F> for RangeCheckerChip<MAX> {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        let mut rows = vec![[F::zero(); NUM_RANGE_COLS]; MAX as usize];
        for (n, row) in rows.iter_mut().enumerate() {
            let cols: &mut RangeCols<F> = unsafe { transmute(row) };
            // FIXME: This is very inefficient when the range is large.
            // Iterate over key/val pairs instead in a separate loop.
            if let Some(c) = self.count.get(&(n as u32)) {
                cols.mult = F::from_canonical_u32(*c);
            }
        }
        RowMajorMatrix::new(rows.concat(), NUM_RANGE_COLS)
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![Interaction {
            fields: vec![VirtualPairCol::single_preprocessed(
                RANGE_PREPROCESSED_COL_MAP.counter,
            )],
            count: VirtualPairCol::single_main(RANGE_COL_MAP.mult),
            argument_index: MachineBus::Range8 as usize,
        }]
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        RangeCols::<F>::headers()
    }
}

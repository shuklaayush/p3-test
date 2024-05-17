use std::collections::BTreeMap;

use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;

use super::{
    columns::{RangeCols, NUM_RANGE_COLS},
    RangeCheckerChip,
};

impl<const MAX: u32> RangeCheckerChip<MAX> {
    pub fn generate_trace<F: PrimeField32>(count: BTreeMap<u32, u32>) -> RowMajorMatrix<F> {
        let num_rows = NUM_RANGE_COLS.next_power_of_two();
        let mut trace =
            RowMajorMatrix::new(vec![F::zero(); num_rows * NUM_RANGE_COLS], NUM_RANGE_COLS);
        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<RangeCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        for (n, row) in rows.iter_mut().enumerate() {
            // FIXME: This is very inefficient when the range is large.
            // Iterate over key/val pairs instead in a separate loop.
            if let Some(c) = count.get(&(n as u32)) {
                row.mult = F::from_canonical_u32(*c);
            }
        }
        trace
    }
}

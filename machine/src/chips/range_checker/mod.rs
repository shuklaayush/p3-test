mod air;
mod columns;
mod interaction;

extern crate alloc;

use alloc::collections::BTreeMap;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_stark::Stark;

use self::columns::{RangeCols, NUM_RANGE_COLS};

#[derive(Default)]
pub struct RangeCheckerChip<const MAX: u32> {
    pub count: BTreeMap<u32, u32>,
    pub bus_range_8: usize,
}

impl<const MAX: u32, F: PrimeField32> Stark<F> for RangeCheckerChip<MAX> {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
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
            if let Some(c) = self.count.get(&(n as u32)) {
                row.mult = F::from_canonical_u32(*c);
            }
        }
        trace
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        RangeCols::<F>::headers()
    }
}

// #[cfg(test)]
// #[cfg(debug_assertions)]
// mod tests {
//     use super::*;
//     use crate::test_util::prove_and_verify;

//     use p3_uni_stark::VerificationError;
//     use rand::random;

//     #[test]
//     fn test_range_prove() -> Result<(), VerificationError> {
//         const NUM: usize = 400;

//         let mut count = BTreeMap::new();
//         for _ in 0..NUM {
//             count
//                 .entry(random::<u8>() as u32)
//                 .and_modify(|c| *c += 1)
//                 .or_insert(1);
//         }
//         let chip = RangeCheckerChip::<256> { count };

//         prove_and_verify(&chip)
//     }
// }

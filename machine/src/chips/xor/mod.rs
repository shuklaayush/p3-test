mod air;
mod columns;
mod interaction;

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_stark::Stark;

use self::columns::{XorCols, NUM_XOR_COLS};

// TODO: Just proof of concept, should be implemented as lookup.
//       Can be extended to a general CPU chip.
pub struct XorChip {
    pub operations: Vec<([u8; 4], [u8; 4])>,
    pub bus_xor_input: usize,
    pub bus_xor_output: usize,
}

impl<F: PrimeField32> Stark<F> for XorChip {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        let num_real_rows = self.operations.len();
        let num_rows = num_real_rows.next_power_of_two();
        let mut trace = RowMajorMatrix::new(vec![F::zero(); num_rows * NUM_XOR_COLS], NUM_XOR_COLS);

        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<XorCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        for (row, (a, b)) in rows.iter_mut().zip(self.operations.iter()) {
            row.is_real = F::one();

            for i in 0..4 {
                row.input1[i] = F::from_canonical_u8(a[i]);
                row.input2[i] = F::from_canonical_u8(b[i]);
                row.output[i] = F::from_canonical_u8(a[i] ^ b[i]);

                for j in 0..8 {
                    row.bits1[i][j] = F::from_canonical_u8(a[i] >> j & 1);
                    row.bits2[i][j] = F::from_canonical_u8(b[i] >> j & 1);
                }
            }
        }

        trace
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        XorCols::<F>::headers()
    }
}

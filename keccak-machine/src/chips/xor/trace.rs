use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;

use super::{
    columns::{XorCols, NUM_XOR_COLS},
    XorChip,
};

impl XorChip {
    pub fn generate_trace<F: PrimeField32>(
        operations: Vec<([u8; 4], [u8; 4])>,
    ) -> RowMajorMatrix<F> {
        let num_real_rows = operations.len();
        let num_rows = num_real_rows.next_power_of_two();
        let mut trace = RowMajorMatrix::new(vec![F::zero(); num_rows * NUM_XOR_COLS], NUM_XOR_COLS);

        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<XorCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        for (row, (a, b)) in rows.iter_mut().zip(operations.iter()) {
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
}

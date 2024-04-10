extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use itertools::Itertools;
use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;

use super::{
    columns::{XorCols, NUM_XOR_COLS, XOR_COL_MAP},
    XorChip,
};
use crate::{chip::Chip, interaction::Interaction, machine::MachineBus};

impl<F: PrimeField64> Chip<F> for XorChip {
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

    // fn sends(&self) -> Vec<Interaction<F>> {
    //     let column_weights = XOR_COL_MAP
    //         .output
    //         .into_iter()
    //         .enumerate()
    //         .map(|(i, c)| (c, F::from_canonical_usize(1 << (8 * i))))
    //         .collect_vec();
    //     vec![Interaction {
    //         fields: vec![VirtualPairCol::new_main(column_weights, F::zero())],
    //         count: VirtualPairCol::single_main(XOR_COL_MAP.is_real),
    //         argument_index: MachineBus::XorOutput as usize,
    //     }]
    // }

    // fn receives(&self) -> Vec<Interaction<F>> {
    //     let vc1 = {
    //         let column_weights = XOR_COL_MAP
    //             .input1
    //             .into_iter()
    //             .enumerate()
    //             .map(|(i, c)| (c, F::from_canonical_usize(1 << (8 * i))))
    //             .collect_vec();
    //         VirtualPairCol::new_main(column_weights, F::zero())
    //     };
    //     let vc2 = {
    //         let column_weights = XOR_COL_MAP
    //             .input2
    //             .into_iter()
    //             .enumerate()
    //             .map(|(i, c)| (c, F::from_canonical_usize(1 << (8 * i))))
    //             .collect_vec();
    //         VirtualPairCol::new_main(column_weights, F::zero())
    //     };
    //     vec![Interaction {
    //         fields: vec![vc1, vc2],
    //         count: VirtualPairCol::single_main(XOR_COL_MAP.is_real),
    //         argument_index: MachineBus::XorInput as usize,
    //     }]
    // }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        XorCols::<F>::headers()
    }
}

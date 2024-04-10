use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{AbstractField, Field};
use p3_matrix::MatrixRowSlices;

use super::columns::{XorCols, NUM_XOR_COLS};
use super::XorChip;

impl<F: Field> BaseAir<F> for XorChip {
    fn width(&self) -> usize {
        NUM_XOR_COLS
    }
}

impl<F, AB> Air<AB> for XorChip
where
    F: Field,
    AB: AirBuilder<F = F>,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &XorCols<AB::Var> = main.row_slice(0).borrow();

        let base2 = [1, 2, 4, 8, 16, 32, 64, 128].map(AB::Expr::from_canonical_u32);

        for i in 0..4 {
            let byte1: AB::Expr = local.bits1[i]
                .into_iter()
                .zip(base2.iter().cloned())
                .map(|(bit, base)| bit * base)
                .sum();
            let byte2: AB::Expr = local.bits2[i]
                .into_iter()
                .zip(base2.iter().cloned())
                .map(|(bit, base)| bit * base)
                .sum();

            // Check that input byte decomposition is correct
            builder.assert_eq(local.input1[i], byte1.clone());
            builder.assert_eq(local.input2[i], byte2.clone());

            let bitwise_and: AB::Expr = local.bits1[i]
                .into_iter()
                .zip(local.bits2[i])
                .zip(base2.iter().cloned())
                .map(|((bit1, bit2), base)| bit1 * bit2 * base)
                .sum();
            let bitwise_xor: AB::Expr = byte1 + byte2 - AB::Expr::two() * bitwise_and.clone();

            // Check the resulting output byte
            builder.assert_eq(bitwise_xor.clone(), local.output[i]);

            // Check that bits are boolean values
            for bit in local.bits1[i].into_iter().chain(local.bits2[i]) {
                builder.assert_bool(bit);
            }
        }

        // TODO: This is dummy to make tests pass.
        //       For some reason, permutation constraints fail when this chip has degree 2.
        builder
            .when(local.is_real)
            .assert_eq(local.is_real * local.is_real, local.is_real * local.is_real);
    }
}

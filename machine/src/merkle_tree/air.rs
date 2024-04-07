use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_keccak_air::U64_LIMBS;
use p3_matrix::MatrixRowSlices;

use super::{
    columns::{MerkleTreeCols, NUM_MERKLE_TREE_COLS},
    MerkleTreeChip,
};
use crate::keccak_permute::NUM_U64_HASH_ELEMS;

impl<F> BaseAir<F> for MerkleTreeChip {
    fn width(&self) -> usize {
        NUM_MERKLE_TREE_COLS
    }
}

impl<AB: AirBuilder> Air<AB> for MerkleTreeChip {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &MerkleTreeCols<AB::Var> = main.row_slice(0).borrow();
        let next: &MerkleTreeCols<AB::Var> = main.row_slice(1).borrow();

        // TODO: Add more constraints.
        builder.assert_bool(local.is_real);

        // Left and right nodes are selected correctly.
        for i in 0..NUM_U64_HASH_ELEMS {
            for j in 0..U64_LIMBS {
                let diff = local.node[i][j] - local.sibling[i][j];
                let left = local.node[i][j] - local.parity_selector * diff.clone();
                let right = local.sibling[i][j] + local.parity_selector * diff;

                builder.assert_eq(left, local.left_node[i][j]);
                builder.assert_eq(right, local.right_node[i][j]);
            }
        }

        // Output is copied to the next row.
        for i in 0..NUM_U64_HASH_ELEMS {
            for j in 0..U64_LIMBS {
                builder
                    .when(AB::Expr::one() - local.is_final_step)
                    .assert_eq(local.output[i][j], next.node[i][j]);
            }
        }

        // TODO: This is dummy to make tests pass.
        //       For some reason, permutation constraints fail when this chip has degree 2.
        builder.when(local.is_real).assert_bool(local.is_real);
    }
}

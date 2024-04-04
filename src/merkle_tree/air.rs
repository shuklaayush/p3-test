use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_matrix::MatrixRowSlices;

use super::{
    columns::{MerkleTreeCols, NUM_MERKLE_TREE_COLS, NUM_U64_HASH_ELEMS, U64_LIMBS},
    MerkleTreeChip,
};

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

        builder.assert_bool(local.is_real);

        let mut builder = builder.when(local.is_real);

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
                    .when_transition()
                    .when_ne(local.is_final_step, AB::Expr::one())
                    .assert_eq(local.output[i][j], next.node[i][j]);
            }
        }
    }
}

use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;
use tracing::instrument;

use super::{
    columns::{MerkleTreeCols, MERKLE_TREE_COL_MAP, NUM_MERKLE_TREE_COLS},
    generation::generate_trace_rows_for_leaf,
    MerkleTreeChip,
};
use crate::{chip::Chip, interaction::Interaction};

impl<F: PrimeField64> Chip<F> for MerkleTreeChip {
    #[instrument(name = "generate MerkleTree trace", skip_all)]
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        let height: usize = self.siblings.iter().map(|s| s.len()).sum();
        let num_rows = height.next_power_of_two();
        let mut trace = RowMajorMatrix::new(
            vec![F::zero(); num_rows * NUM_MERKLE_TREE_COLS],
            NUM_MERKLE_TREE_COLS,
        );
        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<MerkleTreeCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        // TODO: Padding
        for (leaf_rows, ((&leaf, &leaf_index), siblings)) in rows.chunks_mut(height).zip(
            self.leaves
                .iter()
                .zip(&self.leaf_indices)
                .zip(&self.siblings),
        ) {
            generate_trace_rows_for_leaf(leaf_rows, leaf, leaf_index, siblings);
        }

        trace
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        let fields = MERKLE_TREE_COL_MAP
            .left_node
            .into_iter()
            .chain(MERKLE_TREE_COL_MAP.right_node)
            .flatten()
            .map(VirtualPairCol::single_main)
            .collect();
        let is_real = VirtualPairCol::single_main(MERKLE_TREE_COL_MAP.is_real);
        let send = Interaction {
            fields,
            count: is_real,
            argument_index: 1,
        };
        vec![send]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        let fields = MERKLE_TREE_COL_MAP
            .output
            .into_iter()
            .flatten()
            .map(VirtualPairCol::single_main)
            .collect();
        let is_real = VirtualPairCol::single_main(MERKLE_TREE_COL_MAP.is_real);
        let receive = Interaction {
            fields,
            count: is_real,
            argument_index: 0,
        };
        vec![receive]
    }
}

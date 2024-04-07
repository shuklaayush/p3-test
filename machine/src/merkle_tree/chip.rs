use itertools::izip;
use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;
use tracing::instrument;

use super::{
    columns::{MerkleTreeCols, MERKLE_TREE_COL_MAP, NUM_MERKLE_TREE_COLS},
    generation::generate_trace_rows_for_leaf,
    MerkleTreeChip, NUM_U8_HASH_ELEMS,
};
use crate::{chip::Chip, interaction::Interaction};

impl<F: PrimeField64> Chip<F> for MerkleTreeChip {
    // TODO: Allow empty traces
    #[instrument(name = "generate MerkleTree trace", skip_all)]
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        let num_real_rows = self.siblings.iter().map(|s| s.len()).sum::<usize>();
        let num_rows = num_real_rows.next_power_of_two();
        let mut trace = RowMajorMatrix::new(
            vec![F::zero(); num_rows * NUM_MERKLE_TREE_COLS],
            NUM_MERKLE_TREE_COLS,
        );
        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<MerkleTreeCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        let mut offset = 0;
        for (leaf, &leaf_index, siblings) in
            izip!(self.leaves.as_slice(), &self.leaf_indices, &self.siblings)
        {
            let len = siblings.len();
            let leaf_rows = &mut rows[offset..offset + len];
            generate_trace_rows_for_leaf(leaf_rows, leaf, leaf_index, siblings);
            offset += len;

            // TODO: This is unconstrained
            for row in leaf_rows.iter_mut() {
                row.is_real = F::one();
            }
        }

        // Fill padding rows
        for input_rows in rows.chunks_mut(1).skip(num_real_rows) {
            generate_trace_rows_for_leaf(
                input_rows,
                &[0; NUM_U8_HASH_ELEMS],
                0,
                vec![[0; NUM_U8_HASH_ELEMS]].as_slice(),
            );
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

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        MerkleTreeCols::<F>::headers()
    }
}

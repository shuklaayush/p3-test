mod air;
mod columns;
mod generation;
mod interaction;

use itertools::izip;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_stark::Stark;
use tracing::instrument;

use self::{
    columns::{MerkleTreeCols, NUM_MERKLE_TREE_COLS},
    generation::generate_trace_rows_for_leaf,
};

pub(crate) const NUM_U8_HASH_ELEMS: usize = 32;

#[derive(Default)]
pub struct MerkleTreeChip {
    pub leaves: Vec<[u8; NUM_U8_HASH_ELEMS]>,
    pub leaf_indices: Vec<usize>,
    pub siblings: Vec<Vec<[u8; NUM_U8_HASH_ELEMS]>>,

    pub bus_keccak_permute_input: usize,
    pub bus_keccak_digest_output: usize,
}

impl<F: PrimeField32> Stark<F> for MerkleTreeChip {
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

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        MerkleTreeCols::<F>::headers()
    }
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod tests {
    use super::*;
    use crate::test_util::prove_and_verify;

    use itertools::Itertools;
    use p3_keccak::KeccakF;
    use p3_symmetric::{PseudoCompressionFunction, TruncatedPermutation};
    use p3_uni_stark::VerificationError;
    use rand::random;

    fn generate_digests(leaf_hashes: Vec<[u8; 32]>) -> Vec<Vec<[u8; 32]>> {
        let keccak = TruncatedPermutation::new(KeccakF {});
        let mut digests = vec![leaf_hashes];

        while let Some(last_level) = digests.last().cloned() {
            if last_level.len() == 1 {
                break;
            }

            let next_level = last_level
                .chunks_exact(2)
                .map(|chunk| keccak.compress([chunk[0], chunk[1]]))
                .collect();

            digests.push(next_level);
        }

        digests
    }

    #[test]
    fn test_merkle_tree_prove() -> Result<(), VerificationError> {
        const HEIGHT: usize = 3;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32)).map(|_| random()).collect_vec();
        let digests = generate_digests(leaf_hashes);

        let leaf_index = 0;
        let leaf = digests[0][leaf_index];

        let height = digests.len() - 1;
        let siblings = (0..height)
            .map(|i| digests[i][(leaf_index >> i) ^ 1])
            .collect::<Vec<[u8; 32]>>();

        let chip = MerkleTreeChip {
            leaves: vec![leaf],
            leaf_indices: vec![leaf_index],
            siblings: vec![siblings],
            ..Default::default()
        };

        prove_and_verify(&chip, vec![])
    }
}

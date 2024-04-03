mod columns;
mod generation;

use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir, VirtualPairCol};
use p3_field::{AbstractField, PrimeField64};
use p3_matrix::{dense::RowMajorMatrix, MatrixRowSlices};
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::{
    chip::{Chip, Interaction, MachineChip},
    merkle_tree::{columns::MERKLE_TREE_COL_MAP, generation::generate_trace_rows_for_leaf},
};
use columns::{MerkleTreeCols, NUM_MERKLE_TREE_COLS, NUM_U64_HASH_ELEMS, U64_LIMBS};

pub(crate) const NUM_U8_HASH_ELEMS: usize = 32;

pub struct MerkleTreeChip {
    pub leaves: Vec<[u8; NUM_U8_HASH_ELEMS]>,
    pub leaf_indices: Vec<usize>,
    pub siblings: Vec<Vec<[u8; NUM_U8_HASH_ELEMS]>>,
}

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

impl<F: PrimeField64> Chip<F> for MerkleTreeChip {
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

        // TODO:
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

impl<SC: StarkGenericConfig> MachineChip<SC> for MerkleTreeChip where Val<SC>: PrimeField64 {}

#[cfg(test)]
mod tests {
    use super::*;

    use p3_baby_bear::BabyBear;
    use p3_challenger::{HashChallenger, SerializingChallenger32};
    use p3_commit::ExtensionMmcs;
    use p3_dft::Radix2DitParallel;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriConfig, TwoAdicFriPcs};
    use p3_keccak::{Keccak256Hash, KeccakF};
    use p3_matrix::Matrix;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_symmetric::{
        CompressionFunctionFromHasher, PseudoCompressionFunction, SerializingHasher32,
        TruncatedPermutation,
    };
    use p3_uni_stark::{prove, verify, StarkConfig, VerificationError};
    use p3_util::log2_ceil_usize;
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
        type Val = BabyBear;
        type Challenge = BinomialExtensionField<Val, 4>;

        type ByteHash = Keccak256Hash;
        type FieldHash = SerializingHasher32<ByteHash>;
        let byte_hash = ByteHash {};
        let field_hash = FieldHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<u8, ByteHash, 2, 32>;
        let compress = MyCompress::new(byte_hash);

        type ValMmcs = FieldMerkleTreeMmcs<Val, u8, FieldHash, MyCompress, 32>;
        let val_mmcs = ValMmcs::new(field_hash, compress.clone());

        type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
        let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

        type Dft = Radix2DitParallel;
        let dft = Dft {};

        type Challenger = SerializingChallenger32<Val, HashChallenger<u8, ByteHash, 32>>;

        const HEIGHT: usize = 3;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32))
            .map(|_| random())
            .collect::<Vec<_>>();
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
        };
        let trace = chip.generate_trace();

        let fri_config = FriConfig {
            log_blowup: 1,
            num_queries: 100,
            proof_of_work_bits: 16,
            mmcs: challenge_mmcs,
        };
        type Pcs = TwoAdicFriPcs<Val, Dft, ValMmcs, ChallengeMmcs>;
        let pcs = Pcs::new(log2_ceil_usize(trace.height()), dft, val_mmcs, fri_config);

        type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;
        let config = MyConfig::new(pcs);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        let proof = prove(&config, &chip, &mut challenger, trace, &vec![]);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        verify(&config, &chip, &mut challenger, &proof, &vec![])
    }
}

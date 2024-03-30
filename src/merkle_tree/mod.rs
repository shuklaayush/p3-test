mod columns;
mod generation;

use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{AbstractField, PrimeField64};
use p3_matrix::{dense::RowMajorMatrix, MatrixRowSlices};
use std::iter;

use crate::{
    chip::{Chip, Interaction, MachineChip},
    merkle_tree::generation::generate_trace_rows_for_leaf,
};
use columns::{MerkleTreeCols, NUM_MERKLE_TREE_COLS, NUM_U64_HASH_ELEMS, U64_LIMBS};

pub(crate) const NUM_U8_HASH_ELEMS: usize = 32;

pub struct MerkleTreeChip<const HEIGHT: usize> {
    pub leaves: Vec<[u8; NUM_U8_HASH_ELEMS]>,
    pub leaf_indices: Vec<usize>,
    pub siblings: Vec<[[u8; NUM_U8_HASH_ELEMS]; HEIGHT]>,
}

impl<F, const HEIGHT: usize> BaseAir<F> for MerkleTreeChip<HEIGHT> {
    fn width(&self) -> usize {
        NUM_MERKLE_TREE_COLS
    }
}

impl<AB: AirBuilder, const HEIGHT: usize> Air<AB> for MerkleTreeChip<HEIGHT> {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &MerkleTreeCols<AB::Var> = main.row_slice(0).borrow();
        let next: &MerkleTreeCols<AB::Var> = main.row_slice(1).borrow();

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

impl<F: PrimeField64, const HEIGHT: usize> Chip<F> for MerkleTreeChip<HEIGHT> {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        let num_rows = (self.leaves.len() * HEIGHT).next_power_of_two();
        let mut trace = RowMajorMatrix::new(
            vec![F::zero(); num_rows * NUM_MERKLE_TREE_COLS],
            NUM_MERKLE_TREE_COLS,
        );
        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<MerkleTreeCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        for (leaf_rows, ((&leaf, &leaf_index), siblings)) in rows.chunks_mut(HEIGHT).zip(
            self.leaves
                .iter()
                .zip(&self.leaf_indices)
                .zip(&self.siblings)
                .chain(iter::repeat((
                    (&[0; NUM_U8_HASH_ELEMS], &0),
                    &[[0; NUM_U8_HASH_ELEMS]; HEIGHT],
                ))),
        ) {
            generate_trace_rows_for_leaf(leaf_rows, leaf, leaf_index, siblings);
        }

        trace
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![]
    }
}

impl<F: PrimeField64, AB: AirBuilder, const HEIGHT: usize> MachineChip<F, AB>
    for MerkleTreeChip<HEIGHT>
{
}

#[cfg(test)]
mod tests {
    use super::*;

    use p3_baby_bear::BabyBear;
    use p3_challenger::{HashChallenger, SerializingChallenger32};
    use p3_commit::ExtensionMmcs;
    use p3_dft::Radix2DitParallel;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriConfig, TwoAdicFriPcs};
    use p3_keccak::Keccak256Hash;
    use p3_matrix::Matrix;
    use p3_merkle_tree::{FieldMerkleTree, FieldMerkleTreeMmcs};
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::{prove, verify, StarkConfig, VerificationError};
    use p3_util::log2_ceil_usize;
    use rand::random;
    use tracing_forest::util::LevelFilter;
    use tracing_forest::ForestLayer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, Registry};

    const HEIGHT: usize = 3;

    #[test]
    fn test_merkle_tree_prove() -> Result<(), VerificationError> {
        let env_filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();

        Registry::default()
            .with(env_filter)
            .with(ForestLayer::default())
            .init();

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

        let raw_leaves = (0..2u64.pow(HEIGHT as u32))
            .map(|_| random())
            .collect::<Vec<_>>();
        let merkle_tree = FieldMerkleTree::new::<Val, u8, FieldHash, MyCompress>(
            &field_hash,
            &compress,
            vec![RowMajorMatrix::new(raw_leaves, 1)],
        );
        let leaf_index = 0;

        let leaf = merkle_tree.digest_layers[0][leaf_index];
        let siblings = (0..HEIGHT)
            .map(|i| merkle_tree.digest_layers[i][(leaf_index >> i) ^ 1])
            .collect::<Vec<[u8; 32]>>();
        let chip = MerkleTreeChip::<HEIGHT> {
            leaves: vec![leaf],
            leaf_indices: vec![leaf_index],
            siblings: vec![siblings.try_into().unwrap()],
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

        let proof = prove::<MyConfig, _>(&config, &chip, &mut challenger, trace, &vec![]);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        verify(&config, &chip, &mut challenger, &proof, &vec![])
    }
}

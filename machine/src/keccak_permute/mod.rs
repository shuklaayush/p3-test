mod air;
mod chip;
mod columns;
mod generation;
mod round_flags;

use p3_field::PrimeField64;
use p3_uni_stark::StarkGenericConfig;
use p3_uni_stark::Val;

use crate::chip::MachineChip;

pub const NUM_U64_HASH_ELEMS: usize = 4;

/// Assumes the field size is at least 16 bits.
pub struct KeccakPermuteChip {
    pub inputs: Vec<([u64; 25], bool)>,
}

impl<SC: StarkGenericConfig> MachineChip<SC> for KeccakPermuteChip where Val<SC>: PrimeField64 {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chip::Chip;

    use itertools::Itertools;
    use p3_baby_bear::BabyBear;
    use p3_challenger::{HashChallenger, SerializingChallenger32};
    use p3_commit::ExtensionMmcs;
    use p3_dft::Radix2DitParallel;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriConfig, TwoAdicFriPcs};
    use p3_keccak::Keccak256Hash;
    use p3_matrix::Matrix;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::{prove, verify, StarkConfig, VerificationError};
    use p3_util::log2_ceil_usize;
    use rand::random;

    const NUM_HASHES: usize = 10;

    #[test]
    fn test_keccak_prove() -> Result<(), VerificationError> {
        type Val = BabyBear;
        type Challenge = BinomialExtensionField<Val, 4>;

        type ByteHash = Keccak256Hash;
        type FieldHash = SerializingHasher32<ByteHash>;
        let byte_hash = ByteHash {};
        let field_hash = FieldHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<u8, ByteHash, 2, 32>;
        let compress = MyCompress::new(byte_hash);

        type ValMmcs = FieldMerkleTreeMmcs<Val, u8, FieldHash, MyCompress, 32>;
        let val_mmcs = ValMmcs::new(field_hash, compress);

        type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
        let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

        type Dft = Radix2DitParallel;
        let dft = Dft {};

        type Challenger = SerializingChallenger32<Val, HashChallenger<u8, ByteHash, 32>>;

        let inputs = (0..NUM_HASHES).map(|_| random()).collect_vec();
        let chip = KeccakPermuteChip { inputs };
        let trace = chip.generate_trace();

        let fri_config = FriConfig {
            log_blowup: 2,
            num_queries: 42,
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

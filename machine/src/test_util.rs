use crate::chip::{Chip, MachineChip};

use p3_air::Air;
use p3_baby_bear::BabyBear;
use p3_challenger::{FieldChallenger, HashChallenger, SerializingChallenger32};
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2DitParallel;
use p3_field::{extension::BinomialExtensionField, Field, PrimeField64};
use p3_fri::{FriConfig, TwoAdicFriPcs};
use p3_keccak::Keccak256Hash;
use p3_matrix::Matrix;
use p3_merkle_tree::FieldMerkleTreeMmcs;
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
use p3_uni_stark::{
    prove, verify, Proof, ProverConstraintFolder, StarkConfig, StarkGenericConfig,
    SymbolicAirBuilder, VerificationError, VerifierConstraintFolder,
};
use p3_util::log2_ceil_usize;

type Val = BabyBear;
type Challenge = BinomialExtensionField<Val, 4>;
type ByteHash = Keccak256Hash;
type FieldHash = SerializingHasher32<ByteHash>;
type MyCompress = CompressionFunctionFromHasher<u8, ByteHash, 2, 32>;
type ValMmcs = FieldMerkleTreeMmcs<Val, u8, FieldHash, MyCompress, 32>;
type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
type Dft = Radix2DitParallel;
type Challenger = SerializingChallenger32<Val, HashChallenger<u8, ByteHash, 32>>;
type Pcs = TwoAdicFriPcs<Val, Dft, ValMmcs, ChallengeMmcs>;
type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;

pub fn default_config() -> MyConfig {
    let byte_hash = ByteHash {};
    let field_hash = FieldHash::new(Keccak256Hash {});

    let compress = MyCompress::new(byte_hash);

    let val_mmcs = ValMmcs::new(field_hash, compress.clone());

    let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

    let dft = Dft {};

    let fri_config = FriConfig {
        log_blowup: 2,
        num_queries: 42,
        proof_of_work_bits: 16,
        mmcs: challenge_mmcs,
    };
    const MAX_TABLE_HEIGHT: usize = 1024;
    let pcs = Pcs::new(log2_ceil_usize(MAX_TABLE_HEIGHT), dft, val_mmcs, fri_config);

    MyConfig::new(pcs)
}

pub fn default_challenger() -> Challenger {
    let byte_hash = ByteHash {};
    type Challenger = SerializingChallenger32<Val, HashChallenger<u8, ByteHash, 32>>;

    Challenger::from_hasher(vec![], byte_hash)
}

pub(crate) fn prove_and_verify<MC>(chip: &MC) -> Result<(), VerificationError>
where
    p3_uni_stark::Val<MyConfig>: PrimeField64,
    MC: MachineChip<MyConfig>
        + for<'a> Air<ProverConstraintFolder<'a, MyConfig>>
        + for<'a> Air<VerifierConstraintFolder<'a, MyConfig>>
        + for<'a> Air<SymbolicAirBuilder<p3_uni_stark::Val<MyConfig>>>,
{
    let config = default_config();

    let trace = chip.generate_trace();

    let mut challenger = default_challenger();
    let proof = prove(&config, chip, &mut challenger, trace, &vec![]);

    let mut challenger = default_challenger();
    verify(&config, chip, &mut challenger, &proof, &vec![])
}

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
    use crate::test_util::prove_and_verify;

    use itertools::Itertools;
    use p3_uni_stark::VerificationError;
    use rand::random;

    #[test]
    fn test_keccak_prove() -> Result<(), VerificationError> {
        const NUM_HASHES: usize = 10;

        let inputs = (0..NUM_HASHES).map(|_| random()).collect_vec();
        let chip = KeccakPermuteChip { inputs };

        prove_and_verify(&chip)
    }
}

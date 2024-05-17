mod air;
mod columns;
mod interaction;
mod round_flags;
mod trace;

use p3_field::{ExtensionField, PrimeField32};
use p3_stark::AirDebug;

use self::columns::KeccakCols;

pub const NUM_U64_HASH_ELEMS: usize = 4;

/// Assumes the field size is at least 16 bits.
#[derive(Clone, Debug)]
pub struct KeccakPermuteChip {
    pub bus_keccak_permute_input: usize,
    pub bus_keccak_permute_output: usize,
    pub bus_keccak_permute_digest_output: usize,
}

impl<F: PrimeField32, EF: ExtensionField<F>> AirDebug<F, EF> for KeccakPermuteChip {
    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        KeccakCols::<F>::headers()
    }
}

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

        let chip = KeccakPermuteChip {
            bus_keccak_permute_input: 0,
            bus_keccak_permute_output: 0,
            bus_keccak_permute_digest_output: 0,
        };
        let inputs = (0..NUM_HASHES).map(|_| random()).collect_vec();
        let trace = KeccakPermuteChip::generate_trace(inputs);

        prove_and_verify(&chip, trace, vec![])
    }
}

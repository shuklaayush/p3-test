mod air;
mod columns;
mod generation;
mod rap;
mod round_flags;

use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use tracing::instrument;

use self::{columns::KeccakCols, generation::generate_trace_rows};
use crate::chip::Chip;

pub const NUM_U64_HASH_ELEMS: usize = 4;

/// Assumes the field size is at least 16 bits.
pub struct KeccakPermuteChip {
    pub inputs: Vec<([u64; 25], bool)>,
    pub bus_keccak_permute_input: usize,
    pub bus_keccak_permute_output: usize,
    pub bus_keccak_permute_digest_output: usize,
}

impl<F: PrimeField32> Chip<F> for KeccakPermuteChip {
    #[instrument(name = "generate Keccak trace", skip_all)]
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        generate_trace_rows(&self.inputs)
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        KeccakCols::<F>::headers()
    }
}

// #[cfg(test)]
// #[cfg(debug_assertions)]
// mod tests {
//     use super::*;
//     use crate::test_util::prove_and_verify;

//     use itertools::Itertools;
//     use p3_uni_stark::VerificationError;
//     use rand::random;

//     #[test]
//     fn test_keccak_prove() -> Result<(), VerificationError> {
//         const NUM_HASHES: usize = 10;

//         let inputs = (0..NUM_HASHES).map(|_| random()).collect_vec();
//         let chip = KeccakPermuteChip {
//             inputs,
//             bus_keccak_permute_input: 0,
//             bus_keccak_permute_output: 0,
//             bus_keccak_permute_digest_output: 0,
//         };

//         prove_and_verify(&chip)
//     }
// }

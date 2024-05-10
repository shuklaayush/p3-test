mod air;
mod columns;
mod generation;
mod interaction;
mod util;

use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_stark::Stark;
use tracing::instrument;

pub(crate) use self::columns::{KeccakSpongeCols, KECCAK_RATE_BYTES, NUM_KECCAK_SPONGE_COLS};
use self::generation::generate_trace_rows;

/// Strict upper bound for the individual bytes range-check.
const BYTE_RANGE_MAX: usize = 1usize << 8;

#[derive(Default)]
pub struct KeccakSpongeOp {
    pub timestamp: u32,
    pub addr: u32,
    pub input: Vec<u8>,
}

pub struct KeccakSpongeChip {
    pub inputs: Vec<KeccakSpongeOp>,
    pub bus_xor_input: usize,
    pub bus_keccak_permute_input: usize,
    pub bus_range_8: usize,
    pub bus_memory: usize,
    pub bus_xor_output: usize,
    pub bus_keccak_permute_output: usize,
}

impl<F: PrimeField32> Stark<F> for KeccakSpongeChip {
    #[instrument(name = "generate KeccakSponge trace", skip_all)]
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        // Generate the witness row-wise.
        let num_rows = self
            .inputs
            .iter()
            .map(|op| op.input.len() / KECCAK_RATE_BYTES + 1)
            .sum::<usize>()
            .next_power_of_two();
        let mut trace = RowMajorMatrix::new(
            vec![F::zero(); num_rows * NUM_KECCAK_SPONGE_COLS],
            NUM_KECCAK_SPONGE_COLS,
        );
        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<KeccakSpongeCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        generate_trace_rows(rows, self.inputs.as_slice());

        trace
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        KeccakSpongeCols::<F>::headers()
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
//     fn test_keccak_sponge_prove() -> Result<(), VerificationError> {
//         const NUM_BYTES: usize = 400;

//         let op = KeccakSpongeOp {
//             timestamp: 0,
//             addr: 0,
//             input: (0..NUM_BYTES).map(|_| random()).collect_vec(),
//         };
//         let inputs = vec![op];
//         let chip = KeccakSpongeChip { inputs };

//         prove_and_verify(&chip)
//     }
// }

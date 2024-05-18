mod air;
pub mod columns;
mod interaction;
mod trace;
pub mod util;

use p3_field::{ExtensionField, PrimeField32};
use p3_stark::AirDebug;

pub(crate) use self::columns::KeccakSpongeCols;

/// Strict upper bound for the individual bytes range-check.
const BYTE_RANGE_MAX: usize = 1usize << 8;

#[derive(Default, Clone)]
pub struct KeccakSpongeOp {
    pub timestamp: u32,
    pub addr: u32,
    pub input: Vec<u8>,
}

#[derive(Default, Clone, Debug)]
pub struct KeccakSpongeChip {
    pub bus_xor_input: usize,
    pub bus_keccak_permute_input: usize,
    pub bus_range_8: usize,
    pub bus_memory: usize,
    pub bus_xor_output: usize,
    pub bus_keccak_permute_output: usize,
}

impl<F: PrimeField32, EF: ExtensionField<F>> AirDebug<F, EF> for KeccakSpongeChip {
    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        KeccakSpongeCols::<F>::headers()
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
    fn test_keccak_sponge_prove() -> Result<(), VerificationError> {
        const NUM_BYTES: usize = 400;

        let op = KeccakSpongeOp {
            timestamp: 0,
            addr: 0,
            input: (0..NUM_BYTES).map(|_| random()).collect_vec(),
        };
        let inputs = vec![op];
        let trace = KeccakSpongeChip::generate_trace(inputs);
        let chip = KeccakSpongeChip {
            ..Default::default()
        };

        prove_and_verify(&chip, trace, vec![])
    }
}

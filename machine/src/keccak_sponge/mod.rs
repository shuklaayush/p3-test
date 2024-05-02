mod air;
mod chip;
mod columns;
mod generation;
mod util;

pub(crate) use columns::KECCAK_RATE_BYTES;
pub(crate) use util::keccakf_u8s;

use p3_field::PrimeField64;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::chip::MachineChip;

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
}

impl<SC: StarkGenericConfig> MachineChip<SC> for KeccakSpongeChip where Val<SC>: PrimeField64 {}

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
        let chip = KeccakSpongeChip { inputs };

        prove_and_verify(&chip)
    }
}

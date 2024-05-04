mod air;
mod chip;
mod columns;

use p3_field::PrimeField64;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::chip::MachineChip;

pub enum OperationKind {
    Read,
    Write,
}

pub struct MemoryOp {
    pub addr: u32,
    pub timestamp: u32,
    pub value: u8,
    pub kind: OperationKind,
}

pub struct MemoryChip {
    pub operations: Vec<MemoryOp>,
}

impl<SC: StarkGenericConfig> MachineChip<SC> for MemoryChip where Val<SC>: PrimeField64 {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::prove_and_verify;

    use itertools::Itertools;
    use p3_uni_stark::VerificationError;
    use rand::random;

    #[test]
    fn test_memory_prove() -> Result<(), VerificationError> {
        const NUM_BYTES: usize = 400;

        let bytes = (0..NUM_BYTES).map(|_| random()).collect_vec();
        let chip = MemoryChip {
            operations: bytes
                .into_iter()
                .enumerate()
                .map(|(i, b)| MemoryOp {
                    addr: i as u32,
                    timestamp: i as u32,
                    value: b,
                    kind: OperationKind::Read,
                })
                .collect_vec(),
        };

        prove_and_verify(&chip)
    }
}

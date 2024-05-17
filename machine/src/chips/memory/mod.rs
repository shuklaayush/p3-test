mod air;
mod columns;
mod interaction;
mod trace;

use p3_field::{ExtensionField, PrimeField32};
use p3_stark::AirDebug;

use self::columns::MemoryCols;

#[derive(Clone)]
pub enum OperationKind {
    Read,
    Write,
}

#[derive(Clone)]
pub struct MemoryOp {
    pub addr: u32,
    pub timestamp: u32,
    pub value: u8,
    pub kind: OperationKind,
}

#[derive(Default, Clone, Debug)]
pub struct MemoryChip {
    pub bus_memory: usize,
    pub bus_range_8: usize,
}

impl<F: PrimeField32, EF: ExtensionField<F>> AirDebug<F, EF> for MemoryChip {
    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        MemoryCols::<F>::headers()
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
    fn test_memory_prove() -> Result<(), VerificationError> {
        const NUM_BYTES: usize = 400;

        let bytes = (0..NUM_BYTES).map(|_| random()).collect_vec();
        let operations = bytes
            .into_iter()
            .enumerate()
            .map(|(i, b)| MemoryOp {
                addr: i as u32,
                timestamp: i as u32,
                value: b,
                kind: OperationKind::Read,
            })
            .collect_vec();
        let trace = MemoryChip::generate_trace(operations);
        let chip = MemoryChip {
            ..Default::default()
        };

        prove_and_verify(&chip, trace, vec![])
    }
}

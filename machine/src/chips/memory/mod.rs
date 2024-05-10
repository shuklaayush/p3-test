mod air;
mod columns;
mod interaction;

use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_stark::Stark;
use tracing::instrument;

use self::columns::{MemoryCols, NUM_MEMORY_COLS};

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

#[derive(Default)]
pub struct MemoryChip {
    pub operations: Vec<MemoryOp>,

    pub bus_memory: usize,
    pub bus_range_8: usize,
}

impl<F: PrimeField32> Stark<F> for MemoryChip {
    #[instrument(name = "generate Memory trace", skip_all)]
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        let num_real_rows = self.operations.len();
        let num_rows = num_real_rows.next_power_of_two();
        let mut trace =
            RowMajorMatrix::new(vec![F::zero(); num_rows * NUM_MEMORY_COLS], NUM_MEMORY_COLS);

        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<MemoryCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        for (i, (row, op)) in rows.iter_mut().zip(self.operations.iter()).enumerate() {
            row.addr = F::from_canonical_u32(op.addr);
            row.timestamp = F::from_canonical_u32(op.timestamp);
            row.value = F::from_canonical_u8(op.value);

            match op.kind {
                OperationKind::Read => {
                    row.is_read = F::one();
                }
                OperationKind::Write => {
                    row.is_write = F::one();
                }
            }

            if i > 0 {
                let op_prev = &self.operations[i - 1];
                let diff = if op.addr == op_prev.addr {
                    row.addr_unchanged = F::one();
                    op.timestamp - op_prev.timestamp
                } else {
                    op.addr - op_prev.addr - 1
                };
                row.diff_limb_lo = F::from_canonical_u32(diff % (1 << 8));
                row.diff_limb_md = F::from_canonical_u32((diff >> 8) % (1 << 8));
                row.diff_limb_hi = F::from_canonical_u32((diff >> 16) % (1 << 8));
            }
        }

        trace
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        MemoryCols::<F>::headers()
    }
}

#[cfg(test)]
#[cfg(debug_assertions)]
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
            ..Default::default()
        };

        prove_and_verify(&chip, vec![])
    }
}

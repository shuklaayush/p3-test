use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;
use tracing::instrument;

use super::columns::{MemoryCols, NUM_MEMORY_COLS};
use super::{MemoryChip, OperationKind};
use crate::{chip::Chip, interaction::Interaction};

impl<F: PrimeField64> Chip<F> for MemoryChip {
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
            row.is_real = F::one();

            row.addr = F::from_canonical_u32(op.addr);
            row.timestamp = F::from_canonical_u32(op.timestamp);
            row.value = F::from_canonical_u8(op.value);

            if let OperationKind::Read = op.kind {
                row.is_read = F::one();
            }

            if i + 1 < self.operations.len() {
                let op_next = &self.operations[i + 1];
                if op.addr == op_next.addr {
                    row.addr_equal = F::one();
                    row.diff = F::from_canonical_u32(op_next.timestamp - op.timestamp);
                } else {
                    row.diff = F::from_canonical_u32(op_next.addr - op.addr - 1);
                }
            }
        }

        trace
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        MemoryCols::<F>::headers()
    }
}

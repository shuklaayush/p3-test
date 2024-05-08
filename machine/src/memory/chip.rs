use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;
use tracing::instrument;

use super::columns::{MemoryCols, NUM_MEMORY_COLS};
use super::{MemoryChip, OperationKind};
use crate::machine::MachineBus;
use crate::memory::columns::MEMORY_COL_MAP;
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

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![
            // TODO: Combine with is_write?
            Interaction {
                fields: vec![
                    VirtualPairCol::single_main(MEMORY_COL_MAP.timestamp),
                    VirtualPairCol::single_main(MEMORY_COL_MAP.addr),
                    VirtualPairCol::single_main(MEMORY_COL_MAP.value),
                ],
                count: VirtualPairCol::single_main(MEMORY_COL_MAP.is_read),
                argument_index: MachineBus::Memory as usize,
            },
            Interaction {
                fields: vec![VirtualPairCol::single_main(MEMORY_COL_MAP.diff_limb_lo)],
                count: VirtualPairCol::sum_main(vec![
                    MEMORY_COL_MAP.is_read,
                    MEMORY_COL_MAP.is_write,
                ]),
                argument_index: MachineBus::Range8 as usize,
            },
            Interaction {
                fields: vec![VirtualPairCol::single_main(MEMORY_COL_MAP.diff_limb_md)],
                count: VirtualPairCol::sum_main(vec![
                    MEMORY_COL_MAP.is_read,
                    MEMORY_COL_MAP.is_write,
                ]),
                argument_index: MachineBus::Range8 as usize,
            },
            Interaction {
                fields: vec![VirtualPairCol::single_main(MEMORY_COL_MAP.diff_limb_hi)],
                count: VirtualPairCol::sum_main(vec![
                    MEMORY_COL_MAP.is_read,
                    MEMORY_COL_MAP.is_write,
                ]),
                argument_index: MachineBus::Range8 as usize,
            },
        ]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![Interaction {
            fields: vec![
                VirtualPairCol::single_main(MEMORY_COL_MAP.timestamp),
                VirtualPairCol::single_main(MEMORY_COL_MAP.addr),
                VirtualPairCol::single_main(MEMORY_COL_MAP.value),
            ],
            count: VirtualPairCol::single_main(MEMORY_COL_MAP.is_write),
            argument_index: MachineBus::Memory as usize,
        }]
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        MemoryCols::<F>::headers()
    }
}

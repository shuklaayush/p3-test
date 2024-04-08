use itertools::Itertools;
use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_keccak_air::U64_LIMBS;
use p3_matrix::dense::RowMajorMatrix;
use tracing::instrument;

use super::columns::KeccakCols;
use super::generation::generate_trace_rows;
use super::KeccakPermuteChip;
use crate::chip::Chip;
use crate::interaction::Interaction;
use crate::keccak_permute::columns::KECCAK_COL_MAP;
use crate::keccak_permute::NUM_U64_HASH_ELEMS;
use crate::machine::MachineBus;

impl<F: PrimeField64> Chip<F> for KeccakPermuteChip {
    #[instrument(name = "generate Keccak trace", skip_all)]
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        generate_trace_rows(&self.inputs)
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![
            Interaction {
                fields: (0..25)
                    .flat_map(|i| {
                        (0..U64_LIMBS)
                            .map(|limb| KECCAK_COL_MAP.a_prime_prime_prime(i % 5, i / 5, limb))
                            .collect_vec()
                    })
                    .map(VirtualPairCol::single_main)
                    .collect(),
                count: VirtualPairCol::single_main(KECCAK_COL_MAP.is_real_output),
                argument_index: MachineBus::KeccakPermuteOutput as usize,
            },
            Interaction {
                fields: (0..NUM_U64_HASH_ELEMS)
                    .flat_map(|i| {
                        (0..U64_LIMBS)
                            .map(|limb| KECCAK_COL_MAP.a_prime_prime_prime(i % 5, i / 5, limb))
                            .collect_vec()
                    })
                    .map(VirtualPairCol::single_main)
                    .collect(),
                count: VirtualPairCol::single_main(KECCAK_COL_MAP.is_real_digest),
                argument_index: MachineBus::KeccakPermuteDigest as usize,
            },
        ]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![Interaction {
            fields: KECCAK_COL_MAP
                .preimage
                .into_iter()
                .flatten()
                .flatten()
                .map(VirtualPairCol::single_main)
                .collect(),
            count: VirtualPairCol::single_main(KECCAK_COL_MAP.is_real_input),
            argument_index: MachineBus::KeccakPermuteInput as usize,
        }]
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        KeccakCols::<F>::headers()
    }
}

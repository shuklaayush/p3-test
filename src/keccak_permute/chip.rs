use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_keccak_air::{NUM_ROUNDS, U64_LIMBS};
use p3_matrix::dense::RowMajorMatrix;

use super::columns::KECCAK_COL_MAP;
use super::generation::generate_trace_rows;
use super::KeccakPermuteChip;
use crate::chip::Chip;
use crate::interaction::Interaction;
use crate::keccak_permute::NUM_U64_HASH_ELEMS;

impl<F: PrimeField64> Chip<F> for KeccakPermuteChip {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        generate_trace_rows(self.inputs.clone())
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        let fields = (0..NUM_U64_HASH_ELEMS)
            .flat_map(|i| {
                (0..U64_LIMBS)
                    .map(|limb| KECCAK_COL_MAP.a_prime_prime_prime(i % 5, i / 5, limb))
                    .collect::<Vec<_>>()
            })
            .map(VirtualPairCol::single_main)
            .collect();
        let is_real = VirtualPairCol::single_main(KECCAK_COL_MAP.step_flags[NUM_ROUNDS - 1]);
        let send = Interaction {
            fields,
            count: is_real,
            argument_index: 0,
        };
        vec![send]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        let fields = KECCAK_COL_MAP
            .preimage
            .into_iter()
            .flatten()
            .take(2 * NUM_U64_HASH_ELEMS)
            .flatten()
            .map(VirtualPairCol::single_main)
            .collect();
        let is_real = VirtualPairCol::single_main(KECCAK_COL_MAP.step_flags[0]);
        let receive = Interaction {
            fields,
            count: is_real,
            argument_index: 1,
        };
        vec![receive]
    }
}

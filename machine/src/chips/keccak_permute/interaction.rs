use itertools::Itertools;
use p3_air::VirtualPairCol;
use p3_interaction::{Interaction, InteractionAir, InteractionAirBuilder};
use p3_keccak_air::U64_LIMBS;

use super::{columns::KECCAK_COL_MAP, KeccakPermuteChip, NUM_U64_HASH_ELEMS};

impl<AB: InteractionAirBuilder> InteractionAir<AB> for KeccakPermuteChip {
    fn sends(&self) -> Vec<Interaction<AB::Expr>> {
        vec![
            Interaction {
                fields: (0..25)
                    .flat_map(|i| {
                        (0..U64_LIMBS)
                            .map(|limb| {
                                let y = i / 5;
                                let x = i % 5;
                                KECCAK_COL_MAP.a_prime_prime_prime(y, x, limb)
                            })
                            .collect_vec()
                    })
                    .map(VirtualPairCol::single_main)
                    .collect(),
                count: VirtualPairCol::single_main(KECCAK_COL_MAP.is_real_output),
                argument_index: self.bus_keccak_permute_output,
            },
            Interaction {
                fields: (0..NUM_U64_HASH_ELEMS)
                    .flat_map(|i| {
                        (0..U64_LIMBS)
                            .map(|limb| {
                                let y = i / 5;
                                let x = i % 5;
                                KECCAK_COL_MAP.a_prime_prime_prime(y, x, limb)
                            })
                            .collect_vec()
                    })
                    .map(VirtualPairCol::single_main)
                    .collect(),
                count: VirtualPairCol::single_main(KECCAK_COL_MAP.is_real_digest),
                argument_index: self.bus_keccak_permute_digest_output,
            },
        ]
    }

    fn receives(&self) -> Vec<Interaction<AB::Expr>> {
        vec![Interaction {
            fields: KECCAK_COL_MAP
                .preimage
                .into_iter()
                .flatten()
                .flatten()
                .map(VirtualPairCol::single_main)
                .collect(),
            count: VirtualPairCol::single_main(KECCAK_COL_MAP.is_real_input),
            argument_index: self.bus_keccak_permute_input,
        }]
    }
}

use itertools::Itertools;
use p3_air::VirtualPairCol;
use p3_field::AbstractField;
use p3_interaction::{Interaction, InteractionChip};

use super::{
    columns::{KECCAK_RATE_BYTES, KECCAK_SPONGE_COL_MAP},
    KeccakSpongeChip,
};

impl<F: AbstractField> InteractionChip<F> for KeccakSpongeChip {
    fn sends(&self) -> Vec<Interaction<F>> {
        let is_real = VirtualPairCol::sum_main(vec![
            KECCAK_SPONGE_COL_MAP.is_padding_byte[KECCAK_RATE_BYTES - 1],
            KECCAK_SPONGE_COL_MAP.is_full_input_block,
        ]);

        [
            KECCAK_SPONGE_COL_MAP
                .block_bytes
                .chunks(4)
                .zip(KECCAK_SPONGE_COL_MAP.original_rate_u16s.chunks(2))
                .map(|(block, rate)| {
                    let vc1 = {
                        let column_weights = block
                            .iter()
                            .enumerate()
                            .map(|(i, &c)| (c, F::from_canonical_usize(1 << (8 * i))))
                            .collect_vec();
                        VirtualPairCol::new_main(column_weights, F::zero())
                    };
                    let vc2 = {
                        let column_weights = rate
                            .iter()
                            .enumerate()
                            .map(|(i, &c)| (c, F::from_canonical_usize(1 << (16 * i))))
                            .collect_vec();
                        VirtualPairCol::new_main(column_weights, F::zero())
                    };
                    Interaction {
                        fields: vec![vc1, vc2],
                        count: is_real.clone(),
                        argument_index: self.bus_xor_input,
                    }
                })
                .collect_vec(),
            vec![Interaction {
                fields: KECCAK_SPONGE_COL_MAP
                    .xored_rate_u16s
                    .into_iter()
                    .chain(KECCAK_SPONGE_COL_MAP.original_capacity_u16s)
                    .map(VirtualPairCol::single_main)
                    .collect(),
                count: is_real.clone(),
                argument_index: self.bus_keccak_permute_input,
            }],
            (0..KECCAK_RATE_BYTES)
                .map(|i| Interaction {
                    fields: vec![VirtualPairCol::single_main(
                        KECCAK_SPONGE_COL_MAP.block_bytes[i],
                    )],
                    count: is_real.clone(),
                    argument_index: self.bus_range_8,
                })
                .collect_vec(),
        ]
        .concat()
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        let is_real = VirtualPairCol::sum_main(vec![
            KECCAK_SPONGE_COL_MAP.is_padding_byte[KECCAK_RATE_BYTES - 1],
            KECCAK_SPONGE_COL_MAP.is_full_input_block,
        ]);

        // We recover the 16-bit digest limbs from their corresponding bytes,
        // and then append them to the rest of the updated state limbs.
        let mut fields = KECCAK_SPONGE_COL_MAP
            .updated_digest_state_bytes
            .chunks(2)
            .map(|cols| {
                let column_weights = cols
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| (c, F::from_canonical_usize(1 << (8 * i))))
                    .collect_vec();
                VirtualPairCol::new_main(column_weights, F::zero())
            })
            .collect_vec();

        fields.extend(
            KECCAK_SPONGE_COL_MAP
                .partial_updated_state_u16s
                .into_iter()
                .map(VirtualPairCol::single_main),
        );

        [
            (0..KECCAK_RATE_BYTES)
                .map(|i| {
                    let is_real = if i == KECCAK_RATE_BYTES - 1 {
                        VirtualPairCol::single_main(KECCAK_SPONGE_COL_MAP.is_full_input_block)
                    } else {
                        VirtualPairCol::new_main(
                            vec![
                                (KECCAK_SPONGE_COL_MAP.is_full_input_block, F::one()),
                                (
                                    KECCAK_SPONGE_COL_MAP.is_padding_byte[KECCAK_RATE_BYTES - 1],
                                    F::one(),
                                ),
                                (KECCAK_SPONGE_COL_MAP.is_padding_byte[i], -F::one()),
                            ],
                            F::zero(),
                        )
                    };
                    Interaction {
                        fields: vec![
                            VirtualPairCol::single_main(KECCAK_SPONGE_COL_MAP.timestamp),
                            VirtualPairCol::new_main(
                                vec![
                                    (KECCAK_SPONGE_COL_MAP.base_addr, F::one()),
                                    (KECCAK_SPONGE_COL_MAP.already_absorbed_bytes, F::one()),
                                ],
                                F::from_canonical_usize(i),
                            ),
                            VirtualPairCol::single_main(KECCAK_SPONGE_COL_MAP.block_bytes[i]),
                        ],
                        count: is_real,
                        argument_index: self.bus_memory,
                    }
                })
                .collect_vec(),
            KECCAK_SPONGE_COL_MAP
                .xored_rate_u16s
                .chunks(2)
                .map(|rate| {
                    let column_weights = rate
                        .iter()
                        .enumerate()
                        .map(|(i, &c)| (c, F::from_canonical_usize(1 << (16 * i))))
                        .collect_vec();
                    Interaction {
                        fields: vec![VirtualPairCol::new_main(column_weights, F::zero())],
                        count: is_real.clone(),
                        argument_index: self.bus_xor_output,
                    }
                })
                .collect_vec(),
            vec![Interaction {
                fields,
                count: is_real.clone(),
                argument_index: self.bus_keccak_permute_output,
            }],
        ]
        .concat()
    }
}

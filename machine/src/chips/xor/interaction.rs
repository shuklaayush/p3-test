extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use p3_air::VirtualPairCol;
use p3_field::AbstractField;
use p3_interaction::{Interaction, InteractionAir, InteractionAirBuilder, InteractionChip};

use super::{columns::XOR_COL_MAP, XorChip};

impl<F: AbstractField> InteractionChip<F> for XorChip {
    fn sends(&self) -> Vec<Interaction<F>> {
        let column_weights = XOR_COL_MAP
            .output
            .into_iter()
            .enumerate()
            .map(|(i, c)| (c, F::from_canonical_usize(1 << (8 * i))))
            .collect();
        vec![Interaction {
            fields: vec![VirtualPairCol::new_main(column_weights, F::zero())],
            count: VirtualPairCol::single_main(XOR_COL_MAP.is_real),
            argument_index: self.bus_xor_output,
        }]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        let vc1 = {
            let column_weights = XOR_COL_MAP
                .input1
                .into_iter()
                .enumerate()
                .map(|(i, c)| (c, F::from_canonical_usize(1 << (8 * i))))
                .collect();
            VirtualPairCol::new_main(column_weights, F::zero())
        };
        let vc2 = {
            let column_weights = XOR_COL_MAP
                .input2
                .into_iter()
                .enumerate()
                .map(|(i, c)| (c, F::from_canonical_usize(1 << (8 * i))))
                .collect();
            VirtualPairCol::new_main(column_weights, F::zero())
        };
        vec![Interaction {
            fields: vec![vc1, vc2],
            count: VirtualPairCol::single_main(XOR_COL_MAP.is_real),
            argument_index: self.bus_xor_input,
        }]
    }
}

impl<AB: InteractionAirBuilder> InteractionAir<AB> for XorChip {}

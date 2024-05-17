extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use p3_air::VirtualPairCol;
use p3_field::AbstractField;
use p3_interaction::{Interaction, InteractionChip};

use super::{
    columns::{RANGE_COL_MAP, RANGE_PREPROCESSED_COL_MAP},
    RangeCheckerChip,
};

impl<const MAX: u32, F: AbstractField> InteractionChip<F> for RangeCheckerChip<MAX> {
    fn receives(&self) -> Vec<Interaction<F>> {
        vec![Interaction {
            fields: vec![VirtualPairCol::single_preprocessed(
                RANGE_PREPROCESSED_COL_MAP.counter,
            )],
            count: VirtualPairCol::single_main(RANGE_COL_MAP.mult),
            argument_index: self.bus_range_8,
        }]
    }
}

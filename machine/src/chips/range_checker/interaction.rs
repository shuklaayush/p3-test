extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use p3_air::{PairBuilder, VirtualPairCol};
use p3_interaction::{Interaction, InteractionAir, PermutationAirBuilderWithCumulativeSum};

use super::{
    columns::{RANGE_COL_MAP, RANGE_PREPROCESSED_COL_MAP},
    RangeCheckerChip,
};

impl<const MAX: u32, AB: PermutationAirBuilderWithCumulativeSum + PairBuilder> InteractionAir<AB>
    for RangeCheckerChip<MAX>
{
    fn receives(&self) -> Vec<Interaction<AB::Expr>> {
        vec![Interaction {
            fields: vec![VirtualPairCol::single_preprocessed(
                RANGE_PREPROCESSED_COL_MAP.counter,
            )],
            count: VirtualPairCol::single_main(RANGE_COL_MAP.mult),
            argument_index: self.bus_range_8,
        }]
    }
}

mod air;
mod chip;
mod columns;

extern crate alloc;

use alloc::collections::BTreeMap;
use p3_field::PrimeField64;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::chip::MachineChip;

#[derive(Default)]
pub struct RangeCheckerChip<const MAX: u32> {
    pub count: BTreeMap<u32, u32>,
}

impl<SC: StarkGenericConfig, const MAX: u32> MachineChip<SC> for RangeCheckerChip<MAX> where
    Val<SC>: PrimeField64
{
}

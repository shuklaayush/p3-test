mod air;
mod chip;
mod columns;

extern crate alloc;

use p3_field::PrimeField64;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::chip::MachineChip;

// TODO: Just proof of concept, should be implemented as lookup.
//       Can be extended to a general CPU chip.
pub struct XorChip {
    pub operations: Vec<([u8; 4], [u8; 4])>,
}

impl<SC: StarkGenericConfig> MachineChip<SC> for XorChip where Val<SC>: PrimeField64 {}

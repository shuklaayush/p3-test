mod air;
mod columns;
mod interaction;
mod trace;

extern crate alloc;

use alloc::vec::Vec;
use p3_field::PrimeField32;
use p3_stark::Stark;

use self::columns::XorCols;

// TODO: Just proof of concept, should be implemented as lookup.
//       Can be extended to a general CPU chip.
#[derive(Clone)]
pub struct XorChip {
    pub bus_xor_input: usize,
    pub bus_xor_output: usize,
}

impl<F: PrimeField32> Stark<F> for XorChip {
    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        XorCols::<F>::headers()
    }
}

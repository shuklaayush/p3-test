mod air;
mod columns;
mod interaction;
mod trace;

extern crate alloc;

use p3_field::PrimeField32;
use p3_stark::Stark;

use self::columns::RangeCols;

#[derive(Default, Clone)]
pub struct RangeCheckerChip<const MAX: u32> {
    pub bus_range_8: usize,
}

impl<const MAX: u32, F: PrimeField32> Stark<F> for RangeCheckerChip<MAX> {
    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        RangeCols::<F>::headers()
    }
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod tests {
    use super::*;
    use crate::test_util::prove_and_verify;

    use p3_uni_stark::VerificationError;
    use rand::random;
    use std::collections::BTreeMap;

    #[test]
    fn test_range_prove() -> Result<(), VerificationError> {
        const NUM: usize = 400;

        let mut count = BTreeMap::new();
        for _ in 0..NUM {
            count
                .entry(random::<u8>() as u32)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
        let trace = RangeCheckerChip::<256>::generate_trace(count);
        let chip = RangeCheckerChip::<256> {
            ..Default::default()
        };

        prove_and_verify(&chip, trace, vec![])
    }
}

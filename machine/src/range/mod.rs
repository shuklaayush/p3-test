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

#[cfg(test)]
#[cfg(debug_assertions)]
mod tests {
    use super::*;
    use crate::test_util::prove_and_verify;

    use p3_uni_stark::VerificationError;
    use rand::random;

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
        let chip = RangeCheckerChip::<256> { count };

        prove_and_verify(&chip)
    }
}

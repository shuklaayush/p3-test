mod air;
mod chip;
mod columns;
mod generation;

use p3_field::PrimeField64;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::chip::MachineChip;

pub(crate) const NUM_U8_HASH_ELEMS: usize = 32;

pub struct MerkleTreeChip {
    pub leaves: Vec<[u8; NUM_U8_HASH_ELEMS]>,
    pub leaf_indices: Vec<usize>,
    pub siblings: Vec<Vec<[u8; NUM_U8_HASH_ELEMS]>>,
}

impl<SC: StarkGenericConfig> MachineChip<SC> for MerkleTreeChip where Val<SC>: PrimeField64 {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::prove_and_verify;

    use itertools::Itertools;
    use p3_keccak::KeccakF;
    use p3_symmetric::{PseudoCompressionFunction, TruncatedPermutation};
    use p3_uni_stark::VerificationError;
    use rand::random;

    fn generate_digests(leaf_hashes: Vec<[u8; 32]>) -> Vec<Vec<[u8; 32]>> {
        let keccak = TruncatedPermutation::new(KeccakF {});
        let mut digests = vec![leaf_hashes];

        while let Some(last_level) = digests.last().cloned() {
            if last_level.len() == 1 {
                break;
            }

            let next_level = last_level
                .chunks_exact(2)
                .map(|chunk| keccak.compress([chunk[0], chunk[1]]))
                .collect();

            digests.push(next_level);
        }

        digests
    }

    #[test]
    fn test_merkle_tree_prove() -> Result<(), VerificationError> {
        const HEIGHT: usize = 3;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32)).map(|_| random()).collect_vec();
        let digests = generate_digests(leaf_hashes);

        let leaf_index = 0;
        let leaf = digests[0][leaf_index];

        let height = digests.len() - 1;
        let siblings = (0..height)
            .map(|i| digests[i][(leaf_index >> i) ^ 1])
            .collect::<Vec<[u8; 32]>>();

        let chip = MerkleTreeChip {
            leaves: vec![leaf],
            leaf_indices: vec![leaf_index],
            siblings: vec![siblings],
        };

        prove_and_verify(&chip)
    }
}

mod air;
mod columns;
mod interaction;
mod trace;

use p3_field::PrimeField32;
use p3_stark::Stark;

use self::columns::MerkleTreeCols;

pub(crate) const NUM_U8_HASH_ELEMS: usize = 32;

#[derive(Default, Clone, Debug)]
pub struct MerkleTreeChip {
    pub bus_keccak_permute_input: usize,
    pub bus_keccak_digest_output: usize,
}

impl<F: PrimeField32> Stark<F> for MerkleTreeChip {
    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        MerkleTreeCols::<F>::headers()
    }
}

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
        let trace = MerkleTreeChip::generate_trace(vec![leaf], vec![leaf_index], vec![siblings]);

        let chip = MerkleTreeChip {
            ..Default::default()
        };

        prove_and_verify(&chip, trace, vec![])
    }
}

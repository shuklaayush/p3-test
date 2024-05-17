use p3_air::VirtualPairCol;
use p3_field::AbstractField;
use p3_interaction::{Interaction, InteractionChip};

use super::{columns::MERKLE_TREE_COL_MAP, MerkleTreeChip};

impl<F: AbstractField> InteractionChip<F> for MerkleTreeChip {
    fn sends(&self) -> Vec<Interaction<F>> {
        vec![Interaction {
            fields: MERKLE_TREE_COL_MAP
                .left_node
                .into_iter()
                .chain(MERKLE_TREE_COL_MAP.right_node)
                .flatten()
                .map(VirtualPairCol::single_main)
                .collect(),
            count: VirtualPairCol::single_main(MERKLE_TREE_COL_MAP.is_real),
            argument_index: self.bus_keccak_permute_input,
        }]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![Interaction {
            fields: MERKLE_TREE_COL_MAP
                .output
                .into_iter()
                .flatten()
                .map(VirtualPairCol::single_main)
                .collect(),
            count: VirtualPairCol::single_main(MERKLE_TREE_COL_MAP.is_real),
            argument_index: self.bus_keccak_digest_output,
        }]
    }
}

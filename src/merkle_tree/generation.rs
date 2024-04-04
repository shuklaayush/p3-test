use p3_field::PrimeField64;
use p3_keccak::KeccakF;
use p3_symmetric::{PseudoCompressionFunction, TruncatedPermutation};

use super::{
    columns::{MerkleTreeCols, NUM_U64_HASH_ELEMS, U64_LIMBS},
    NUM_U8_HASH_ELEMS,
};

pub fn generate_trace_rows_for_leaf<F: PrimeField64>(
    rows: &mut [MerkleTreeCols<F>],
    leaf: [u8; NUM_U8_HASH_ELEMS],
    leaf_index: usize,
    siblings: &[[u8; NUM_U8_HASH_ELEMS]],
) {
    // Fill the first row with the leaf.
    for (x, input) in leaf
        .chunks(NUM_U8_HASH_ELEMS / NUM_U64_HASH_ELEMS)
        .enumerate()
    {
        for limb in 0..U64_LIMBS {
            let limb_range = limb * 2..(limb + 1) * 2;
            rows[0].node[x][limb] =
                F::from_canonical_u16(u16::from_le_bytes(input[limb_range].try_into().unwrap()));
        }
    }

    let mut node = generate_trace_row_for_round(&mut rows[0], leaf_index & 1, leaf, siblings[0]);

    for round in 1..rows.len() {
        // Copy previous row's output to next row's input.
        for x in 0..NUM_U64_HASH_ELEMS {
            for limb in 0..U64_LIMBS {
                rows[round].node[x][limb] = rows[round - 1].output[x][limb];
            }
        }

        node = generate_trace_row_for_round(
            &mut rows[round],
            (leaf_index >> round) & 1,
            node,
            siblings[round],
        );
    }

    // Set the final step flag.
    rows[rows.len() - 1].is_final_step = F::one();
}

pub fn generate_trace_row_for_round<F: PrimeField64>(
    row: &mut MerkleTreeCols<F>,
    parity_bit: usize,
    node: [u8; NUM_U8_HASH_ELEMS],
    sibling: [u8; NUM_U8_HASH_ELEMS],
) -> [u8; NUM_U8_HASH_ELEMS] {
    row.is_real = F::one();

    let (left_node, right_node) = if parity_bit == 0 {
        (node, sibling)
    } else {
        (sibling, node)
    };

    let keccak = TruncatedPermutation::new(KeccakF {});
    let output = keccak.compress([left_node, right_node]);

    row.parity_selector = F::from_canonical_usize(parity_bit);
    for x in 0..NUM_U64_HASH_ELEMS {
        let offset = x * NUM_U8_HASH_ELEMS / NUM_U64_HASH_ELEMS;
        for limb in 0..U64_LIMBS {
            let limb_range = (offset + limb * 2)..(offset + (limb + 1) * 2);

            row.sibling[x][limb] = F::from_canonical_u16(u16::from_le_bytes(
                sibling[limb_range.clone()].try_into().unwrap(),
            ));

            row.left_node[x][limb] = F::from_canonical_u16(u16::from_le_bytes(
                left_node[limb_range.clone()].try_into().unwrap(),
            ));
            row.right_node[x][limb] = F::from_canonical_u16(u16::from_le_bytes(
                right_node[limb_range.clone()].try_into().unwrap(),
            ));

            row.output[x][limb] =
                F::from_canonical_u16(u16::from_le_bytes(output[limb_range].try_into().unwrap()));
        }
    }

    output
}

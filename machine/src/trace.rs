use std::collections::BTreeMap;

use itertools::Itertools;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::chips::{
    keccak_permute::KeccakPermuteChip,
    keccak_sponge::{
        columns::KECCAK_RATE_BYTES, util::keccakf_u8s, KeccakSpongeChip, KeccakSpongeOp,
    },
    memory::{MemoryChip, MemoryOp, OperationKind},
    merkle_tree::MerkleTreeChip,
    range_checker::RangeCheckerChip,
    xor::XorChip,
};

// TODO: Proper execution function for the machine that minimizes redundant computation
// Store logs/events during execution first and then generate the traces
pub fn generate_machine_trace<SC: StarkGenericConfig>(
    preimage_bytes: Vec<u8>,
    digests: Vec<Vec<[u8; 32]>>,
    leaf_index: usize,
) -> Vec<Option<RowMajorMatrix<Val<SC>>>>
where
    Val<SC>: PrimeField32,
{
    let leaf = digests[0][leaf_index];

    let height = digests.len() - 1;
    let siblings = (0..height)
        .map(|i| digests[i][(leaf_index >> i) ^ 1])
        .collect::<Vec<[u8; 32]>>();
    let mut keccak_inputs = (0..height)
        .map(|i| {
            let index = leaf_index >> i;
            let parity = index & 1;
            let (left, right) = if parity == 0 {
                (digests[i][index], digests[i][index ^ 1])
            } else {
                (digests[i][index ^ 1], digests[i][index])
            };
            let mut input = [0; 25];
            input[0..4].copy_from_slice(
                left.chunks_exact(8)
                    .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                    .collect_vec()
                    .as_slice(),
            );
            input[4..8].copy_from_slice(
                right
                    .chunks_exact(8)
                    .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                    .collect_vec()
                    .as_slice(),
            );
            (input, true)
        })
        .collect_vec();

    let merkle_tree_trace =
        MerkleTreeChip::generate_trace(vec![leaf], vec![leaf_index], vec![siblings]);

    let keccak_sponge_trace = KeccakSpongeChip::generate_trace(vec![KeccakSpongeOp {
        timestamp: 0,
        addr: 0,
        input: preimage_bytes.clone(),
    }]);

    let memory_ops = preimage_bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| MemoryOp {
            addr: i as u32,
            // TODO: Use proper timestamp
            timestamp: 0,
            value: b,
            kind: OperationKind::Read,
        })
        .collect_vec();
    let memory_trace = MemoryChip::generate_trace(memory_ops.clone());

    let preimage_len = preimage_bytes.len();

    let mut padded_preimage = preimage_bytes.clone();
    let padding_len = KECCAK_RATE_BYTES - (preimage_len % KECCAK_RATE_BYTES);
    padded_preimage.resize(preimage_len + padding_len, 0);
    padded_preimage[preimage_len] = 1;
    *padded_preimage.last_mut().unwrap() |= 0b10000000;

    let mut xor_inputs = Vec::new();

    let mut state = [0u8; 200];
    let keccak_inputs_full = padded_preimage
        .chunks(KECCAK_RATE_BYTES)
        .map(|b| {
            state[..KECCAK_RATE_BYTES]
                .chunks(4)
                .zip_eq(b.chunks(4))
                .for_each(|(s, b)| {
                    xor_inputs.push((b.try_into().unwrap(), s.try_into().unwrap()));
                });
            state[..KECCAK_RATE_BYTES]
                .iter_mut()
                .zip_eq(b.iter())
                .for_each(|(s, b)| {
                    *s ^= *b;
                });
            let input: [u64; 25] = state
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                .collect_vec()
                .try_into()
                .unwrap();

            keccakf_u8s(&mut state);
            input
        })
        .collect_vec();
    keccak_inputs.extend(keccak_inputs_full.into_iter().map(|input| (input, false)));

    let keccak_permute_trace = KeccakPermuteChip::generate_trace(keccak_inputs);

    let mut range_counts = BTreeMap::new();
    // TODO: This is wrong, should be just the preimage
    for byte in padded_preimage {
        range_counts
            .entry(byte as u32)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }
    for (i, op) in memory_ops.iter().enumerate() {
        let diff = if i > 0 {
            let op_prev = &memory_ops[i - 1];
            if op.addr == op_prev.addr {
                op.timestamp - op_prev.timestamp
            } else {
                op.addr - op_prev.addr - 1
            }
        } else {
            0
        };
        let diff_limb_lo = diff % (1 << 8);
        let diff_limb_md = (diff >> 8) % (1 << 8);
        let diff_limb_hi = (diff >> 16) % (1 << 8);

        range_counts
            .entry(diff_limb_lo)
            .and_modify(|c| *c += 1)
            .or_insert(1);
        range_counts
            .entry(diff_limb_md)
            .and_modify(|c| *c += 1)
            .or_insert(1);
        range_counts
            .entry(diff_limb_hi)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    let range_trace = RangeCheckerChip::<256>::generate_trace(range_counts);

    let xor_trace = XorChip::generate_trace(xor_inputs);

    vec![
        Some(keccak_permute_trace),
        Some(keccak_sponge_trace),
        Some(merkle_tree_trace),
        Some(range_trace),
        Some(xor_trace),
        Some(memory_trace),
    ]
}

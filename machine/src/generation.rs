// TODO: Proper execution function for the machine that minimizes redundant computation
//       Store logs/events during execution first and then generate the traces
// pub fn new(preimage_bytes: Vec<u8>, digests: Vec<Vec<[u8; 32]>>, leaf_index: usize) -> Self {
//     let leaf = digests[0][leaf_index];

//     let height = digests.len() - 1;
//     let siblings = (0..height)
//         .map(|i| digests[i][(leaf_index >> i) ^ 1])
//         .collect::<Vec<[u8; 32]>>();
//     let mut keccak_inputs = (0..height)
//         .map(|i| {
//             let index = leaf_index >> i;
//             let parity = index & 1;
//             let (left, right) = if parity == 0 {
//                 (digests[i][index], digests[i][index ^ 1])
//             } else {
//                 (digests[i][index ^ 1], digests[i][index])
//             };
//             let mut input = [0; 25];
//             input[0..4].copy_from_slice(
//                 left.chunks_exact(8)
//                     .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
//                     .collect_vec()
//                     .as_slice(),
//             );
//             input[4..8].copy_from_slice(
//                 right
//                     .chunks_exact(8)
//                     .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
//                     .collect_vec()
//                     .as_slice(),
//             );
//             (input, true)
//         })
//         .collect_vec();

//     let merkle_tree_chip = MerkleTreeChip {
//         leaves: vec![leaf],
//         leaf_indices: vec![leaf_index],
//         siblings: vec![siblings],
//     };

//     let keccak_sponge_chip = KeccakSpongeChip {
//         inputs: vec![KeccakSpongeOp {
//             timestamp: 0,
//             addr: 0,
//             input: preimage_bytes.clone(),
//         }],
//     };

//     let memory_ops = preimage_bytes
//         .iter()
//         .enumerate()
//         .map(|(i, &b)| MemoryOp {
//             addr: i as u32,
//             // TODO: Use proper timestamp
//             timestamp: 0,
//             value: b,
//             kind: OperationKind::Read,
//         })
//         .collect_vec();
//     let memory_chip = MemoryChip {
//         operations: memory_ops.clone(),
//     };

//     let preimage_len = preimage_bytes.len();

//     let mut padded_preimage = preimage_bytes.clone();
//     let padding_len = KECCAK_RATE_BYTES - (preimage_len % KECCAK_RATE_BYTES);
//     padded_preimage.resize(preimage_len + padding_len, 0);
//     padded_preimage[preimage_len] = 1;
//     *padded_preimage.last_mut().unwrap() |= 0b10000000;

//     let mut xor_inputs = Vec::new();

//     let mut state = [0u8; 200];
//     let keccak_inputs_full = padded_preimage
//         .chunks(KECCAK_RATE_BYTES)
//         .map(|b| {
//             state[..KECCAK_RATE_BYTES]
//                 .chunks(4)
//                 .zip_eq(b.chunks(4))
//                 .for_each(|(s, b)| {
//                     xor_inputs.push((b.try_into().unwrap(), s.try_into().unwrap()));
//                 });
//             state[..KECCAK_RATE_BYTES]
//                 .iter_mut()
//                 .zip_eq(b.iter())
//                 .for_each(|(s, b)| {
//                     *s ^= *b;
//                 });
//             let input: [u64; 25] = state
//                 .chunks_exact(8)
//                 .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
//                 .collect_vec()
//                 .try_into()
//                 .unwrap();

//             keccakf_u8s(&mut state);
//             input
//         })
//         .collect_vec();
//     keccak_inputs.extend(keccak_inputs_full.into_iter().map(|input| (input, false)));

//     let keccak_permute_chip = KeccakPermuteChip {
//         inputs: keccak_inputs,
//     };

//     let mut range_counts = BTreeMap::new();
//     // TODO: This is wrong, should be just the preimage
//     for byte in padded_preimage {
//         range_counts
//             .entry(byte as u32)
//             .and_modify(|c| *c += 1)
//             .or_insert(1);
//     }
//     for (i, op) in memory_ops.iter().enumerate() {
//         let diff = if i > 0 {
//             let op_prev = &memory_ops[i - 1];
//             if op.addr == op_prev.addr {
//                 op.timestamp - op_prev.timestamp
//             } else {
//                 op.addr - op_prev.addr - 1
//             }
//         } else {
//             0
//         };
//         let diff_limb_lo = diff % (1 << 8);
//         let diff_limb_md = (diff >> 8) % (1 << 8);
//         let diff_limb_hi = (diff >> 16) % (1 << 8);

//         range_counts
//             .entry(diff_limb_lo)
//             .and_modify(|c| *c += 1)
//             .or_insert(1);
//         range_counts
//             .entry(diff_limb_md)
//             .and_modify(|c| *c += 1)
//             .or_insert(1);
//         range_counts
//             .entry(diff_limb_hi)
//             .and_modify(|c| *c += 1)
//             .or_insert(1);
//     }

//     let range_chip = RangeCheckerChip {
//         count: range_counts,
//     };

//     let xor_chip = XorChip {
//         operations: xor_inputs,
//     };

//     Self {
//         keccak_permute_chip: ChipType::KeccakPermute(keccak_permute_chip),
//         keccak_sponge_chip: ChipType::KeccakSponge(keccak_sponge_chip),
//         merkle_tree_chip: ChipType::MerkleTree(merkle_tree_chip),
//         range_chip: ChipType::Range8(range_chip),
//         xor_chip: ChipType::Xor(xor_chip),
//         memory_chip: ChipType::Memory(memory_chip),
//     }
// }

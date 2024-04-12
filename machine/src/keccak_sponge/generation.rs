use itertools::Itertools;
use p3_field::PrimeField64;

use super::{
    columns::{KECCAK_DIGEST_U16S, KECCAK_RATE_BYTES, KECCAK_RATE_U16S, KECCAK_WIDTH_U16S},
    util::keccakf_u16s,
};
use crate::keccak_sponge::columns::KeccakSpongeCols;

pub fn generate_trace_rows<F: PrimeField64>(rows: &mut [KeccakSpongeCols<F>], inputs: &[Vec<u8>]) {
    let mut offset = 0;
    for input in inputs {
        let len = input.len() / KECCAK_RATE_BYTES + 1;
        let input_rows = &mut rows[offset..offset + len];
        generate_rows_for_input(input_rows, input);
        offset += len;
    }

    // Pad the trace.
    for input_rows in rows.chunks_mut(1).skip(offset) {
        generate_rows_for_input(input_rows, vec![].as_slice());
    }
}

/// Generates the rows associated to a given operation:
/// Performs a Keccak sponge permutation and fills the STARK's rows
/// accordingly. The number of rows is the number of input chunks of
/// size `KECCAK_RATE_BYTES`.
fn generate_rows_for_input<F: PrimeField64>(rows: &mut [KeccakSpongeCols<F>], input: &[u8]) {
    let mut sponge_state = [0u16; KECCAK_WIDTH_U16S];

    let mut input_blocks = input.chunks_exact(KECCAK_RATE_BYTES);
    let mut already_absorbed_bytes = 0;
    for (row, block) in rows.iter_mut().zip(input_blocks.by_ref()) {
        // We compute the updated state of the sponge.
        generate_full_input_row(
            row,
            already_absorbed_bytes,
            sponge_state,
            block.try_into().unwrap(),
        );

        // We update the state limbs for the next block absorption.
        // The first `KECCAK_DIGEST_U16s` limbs are stored as bytes after the
        // computation, so we recompute the corresponding `u16` and update
        // the first state limbs.
        sponge_state[..KECCAK_DIGEST_U16S]
            .iter_mut()
            .zip(row.updated_digest_state_bytes.chunks_exact(2))
            .for_each(|(s, bs)| {
                *s = bs
                    .iter()
                    .enumerate()
                    .map(|(i, b)| (b.as_canonical_u64() as u16) << (8 * i))
                    .sum();
            });

        // The rest of the bytes are already stored in the expected form, so we can
        // directly update the state with the stored values.
        sponge_state[KECCAK_DIGEST_U16S..]
            .iter_mut()
            .zip(row.partial_updated_state_u16s)
            .for_each(|(s, x)| *s = x.as_canonical_u64() as u16);

        already_absorbed_bytes += KECCAK_RATE_BYTES;
    }

    generate_final_row(
        rows.last_mut().unwrap(),
        input,
        already_absorbed_bytes,
        sponge_state,
        input_blocks.remainder(),
    );
}

/// Generates a row where all bytes are input bytes, not padding bytes.
/// This includes updating the state sponge with a single absorption.
fn generate_full_input_row<F: PrimeField64>(
    row: &mut KeccakSpongeCols<F>,
    already_absorbed_bytes: usize,
    sponge_state: [u16; KECCAK_WIDTH_U16S],
    block: [u8; KECCAK_RATE_BYTES],
) {
    // TODO: This is unconstrained
    row.is_full_input_block = F::one();
    row.block_bytes = block.map(F::from_canonical_u8);

    generate_common_fields(row, already_absorbed_bytes, sponge_state);
}

/// Generates a row containing the last input bytes.
/// On top of computing one absorption and padding the input,
/// we indicate the last non-padding input byte by setting
/// `row.is_final_input_len[final_inputs.len()]` to 1.
fn generate_final_row<F: PrimeField64>(
    row: &mut KeccakSpongeCols<F>,
    input: &[u8],
    already_absorbed_bytes: usize,
    sponge_state: [u16; KECCAK_WIDTH_U16S],
    final_inputs: &[u8],
) {
    assert_eq!(already_absorbed_bytes + final_inputs.len(), input.len());

    for (block_byte, input_byte) in row.block_bytes.iter_mut().zip(final_inputs) {
        *block_byte = F::from_canonical_u8(*input_byte);
    }

    // pad10*1 rule
    if final_inputs.len() == KECCAK_RATE_BYTES - 1 {
        // Both 1s are placed in the same byte.
        row.block_bytes[final_inputs.len()] = F::from_canonical_u8(0b10000001);
    } else {
        row.block_bytes[final_inputs.len()] = F::one();
        row.block_bytes[KECCAK_RATE_BYTES - 1] = F::from_canonical_u8(0b10000000);
    }

    row.is_final_input_len[final_inputs.len()] = F::one();

    generate_common_fields(row, already_absorbed_bytes, sponge_state)
}

/// Generate fields that are common to both full-input-block rows and
/// final-block rows. Also updates the sponge state with a single
/// absorption. Given a state S = R || C and a block input B,
/// - R is updated with R XOR B,
/// - S is replaced by keccakf_u16s(S).
fn generate_common_fields<F: PrimeField64>(
    row: &mut KeccakSpongeCols<F>,
    already_absorbed_bytes: usize,
    mut sponge_state: [u16; KECCAK_WIDTH_U16S],
) {
    row.already_absorbed_bytes = F::from_canonical_usize(already_absorbed_bytes);

    row.original_rate_u16s = sponge_state[..KECCAK_RATE_U16S]
        .iter()
        .map(|x| F::from_canonical_u16(*x))
        .collect_vec()
        .try_into()
        .unwrap();

    row.original_capacity_u16s = sponge_state[KECCAK_RATE_U16S..]
        .iter()
        .map(|x| F::from_canonical_u16(*x))
        .collect_vec()
        .try_into()
        .unwrap();

    let block_u16s = (0..KECCAK_RATE_U16S).map(|i| {
        u16::from_le_bytes(
            row.block_bytes[i * 2..(i + 1) * 2]
                .iter()
                .map(|x| x.as_canonical_u64() as u8)
                .collect_vec()
                .try_into()
                .unwrap(),
        )
    });

    // xor in the block
    for (state_i, block_i) in sponge_state.iter_mut().zip(block_u16s) {
        *state_i ^= block_i;
    }
    let xored_rate_u16s: [u16; KECCAK_RATE_U16S] = sponge_state[..KECCAK_RATE_U16S]
        .to_vec()
        .try_into()
        .unwrap();
    row.xored_rate_u16s = xored_rate_u16s.map(F::from_canonical_u16);

    keccakf_u16s(&mut sponge_state);
    // Store all but the first `KECCAK_DIGEST_U16S` limbs in the updated state.
    // Those missing limbs will be broken down into bytes and stored separately.
    row.partial_updated_state_u16s.copy_from_slice(
        &sponge_state[KECCAK_DIGEST_U16S..]
            .iter()
            .copied()
            .map(|i| F::from_canonical_u16(i))
            .collect_vec(),
    );
    sponge_state[..KECCAK_DIGEST_U16S]
        .iter()
        .enumerate()
        .for_each(|(l, &elt)| {
            let mut cur_elt = elt;
            (0..2).for_each(|i| {
                row.updated_digest_state_bytes[l * 2 + i] = F::from_canonical_u16(cur_elt & 0xFF);
                cur_elt >>= 8;
            });

            // 16-bit limb reconstruction consistency check.
            let mut s = row.updated_digest_state_bytes[l * 2].as_canonical_u64();
            for i in 1..2 {
                s += row.updated_digest_state_bytes[l * 2 + i].as_canonical_u64() << (8 * i);
            }
            assert_eq!(elt as u64, s, "not equal");
        })
}

/// Expects input in *column*-major layout
pub fn generate_range_checks<F: PrimeField64>(_rows: &mut [KeccakSpongeCols<F>]) {
    // for i in 0..BYTE_RANGE_MAX {
    //     rows[i].range_counter = F::from_canonical_usize(i);
    // }
    // for i in BYTE_RANGE_MAX..rows.len() {
    //     rows[i].range_counter = F::from_canonical_usize(BYTE_RANGE_MAX - 1);
    // }

    // // For each column c in cols, generate the range-check
    // // permutations and put them in the corresponding range-check
    // // columns rc_c and rc_c+1.
    // for j in 0..KECCAK_RATE_BYTES {
    //     for i in 0..rows.len() {
    //         let x = rows[i].block_bytes[j].as_canonical_u64() as usize;
    //         assert!(
    //             x < BYTE_RANGE_MAX,
    //             "column value {} exceeds the max range value {}",
    //             x,
    //             BYTE_RANGE_MAX
    //         );
    //         rows[x].rc_frequencies += F::one();
    //     }
    // }
}

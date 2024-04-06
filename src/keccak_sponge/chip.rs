use p3_air::VirtualPairCol;
use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;
use tracing::instrument;

use super::{
    columns::{KeccakSpongeCols, KECCAK_RATE_BYTES, NUM_KECCAK_SPONGE_COLS},
    generation::{generate_range_checks, generate_trace_rows},
    KeccakSpongeChip,
};
use crate::{chip::Chip, interaction::Interaction, keccak_sponge::columns::KECCAK_SPONGE_COL_MAP};

impl<F: PrimeField64> Chip<F> for KeccakSpongeChip {
    #[instrument(name = "generate KeccakSponge trace", skip_all)]
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        // Generate the witness row-wise.
        let num_rows = self
            .inputs
            .iter()
            .map(|input| input.len() / KECCAK_RATE_BYTES + 1)
            .sum::<usize>()
            .next_power_of_two();
        let mut trace = RowMajorMatrix::new(
            vec![F::zero(); num_rows * NUM_KECCAK_SPONGE_COLS],
            NUM_KECCAK_SPONGE_COLS,
        );
        let (prefix, rows, suffix) = unsafe { trace.values.align_to_mut::<KeccakSpongeCols<F>>() };
        assert!(prefix.is_empty(), "Alignment should match");
        assert!(suffix.is_empty(), "Alignment should match");
        assert_eq!(rows.len(), num_rows);

        generate_trace_rows(rows, self.inputs.as_slice());

        generate_range_checks(rows);

        println!("Trace: {:?}", trace);

        trace
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![]
    }
}

/*
/// Creates the vector of `Columns` corresponding to:
/// - the address in memory of the inputs,
/// - the length of the inputs,
/// - the timestamp at which the inputs are read from memory,
/// - the output limbs of the Keccak sponge.
pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;
    let mut outputs = Vec::with_capacity(8);
    for i in (0..8).rev() {
        let cur_col = Column::linear_combination(
            cols.updated_digest_state_bytes[i * 4..(i + 1) * 4]
                .iter()
                .enumerate()
                .map(|(j, &c)| (c, F::from_canonical_u64(1 << (24 - 8 * j)))),
        );
        outputs.push(cur_col);
    }

    // The length of the inputs is `already_absorbed_bytes + is_final_input_len`.
    let len_col = Column::linear_combination(
        iter::once((cols.already_absorbed_bytes, F::ONE)).chain(
            cols.is_final_input_len
                .iter()
                .enumerate()
                .map(|(i, &elt)| (elt, F::from_canonical_usize(i))),
        ),
    );

    let mut res: Vec<Column<F>> =
        Column::singles([cols.context, cols.segment, cols.virt]).collect();
    res.push(len_col);
    res.push(Column::single(cols.timestamp));
    res.extend(outputs);

    res
}

/// Creates the vector of `Columns` corresponding to the inputs of the Keccak
/// sponge. This is used to check that the inputs of the sponge correspond to
/// the inputs given by `KeccakStark`.
pub(crate) fn ctl_looking_keccak_inputs<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;
    let mut res: Vec<_> = Column::singles(
        [
            cols.xored_rate_u32s.as_slice(),
            &cols.original_capacity_u32s,
        ]
        .concat(),
    )
    .collect();
    res.push(Column::single(cols.timestamp));

    res
}

/// Creates the vector of `Columns` corresponding to the outputs of the Keccak
/// sponge. This is used to check that the outputs of the sponge correspond to
/// the outputs given by `KeccakStark`.
pub(crate) fn ctl_looking_keccak_outputs<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;

    // We recover the 32-bit digest limbs from their corresponding bytes,
    // and then append them to the rest of the updated state limbs.
    let digest_u32s = cols.updated_digest_state_bytes.chunks_exact(4).map(|c| {
        Column::linear_combination(
            c.iter()
                .enumerate()
                .map(|(i, &b)| (b, F::from_canonical_usize(1 << (8 * i)))),
        )
    });

    let mut res: Vec<_> = digest_u32s.collect();

    res.extend(Column::singles(&cols.partial_updated_state_u32s));
    res.push(Column::single(cols.timestamp));

    res
}

/// Creates the vector of `Columns` corresponding to the address and value of
/// the byte being read from memory.
pub(crate) fn ctl_looking_memory<F: Field>(i: usize) -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;

    let mut res = vec![Column::constant(F::ONE)]; // is_read

    res.extend(Column::singles([cols.context, cols.segment]));

    // The address of the byte being read is `virt + already_absorbed_bytes + i`.
    res.push(Column::linear_combination_with_constant(
        [(cols.virt, F::ONE), (cols.already_absorbed_bytes, F::ONE)],
        F::from_canonical_usize(i),
    ));

    // The i'th input byte being read.
    res.push(Column::single(cols.block_bytes[i]));

    // Since we're reading a single byte, the higher limbs must be zero.
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(cols.timestamp));

    assert_eq!(
        res.len(),
        crate::memory::memory_stark::ctl_data::<F>().len()
    );
    res
}

/// Returns the number of `KeccakSponge` tables looking into the `LogicStark`.
pub(crate) const fn num_logic_ctls() -> usize {
    const U8S_PER_CTL: usize = 32;
    ceil_div_usize(KECCAK_RATE_BYTES, U8S_PER_CTL)
}

/// Creates the vector of `Columns` required to perform the `i`th logic CTL.
/// It is comprised of the ÌS_XOR` flag, the two inputs and the output
/// of the XOR operation.
/// Since we need to do 136 byte XORs, and the logic CTL can
/// XOR 32 bytes per CTL, there are 5 such CTLs.
pub(crate) fn ctl_looking_logic<F: Field>(i: usize) -> Vec<Column<F>> {
    const U32S_PER_CTL: usize = 8;
    const U8S_PER_CTL: usize = 32;

    debug_assert!(i < num_logic_ctls());
    let cols = KECCAK_SPONGE_COL_MAP;

    let mut res = vec![
        Column::constant(F::from_canonical_u8(0x18)), // is_xor
    ];

    // Input 0 contains some of the sponge's original rate chunks. If this is the
    // last CTL, we won't need to use all of the CTL's inputs, so we will pass
    // some zeros.
    res.extend(
        Column::singles(&cols.original_rate_u32s[i * U32S_PER_CTL..])
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    // Input 1 contains some of block's chunks. Again, for the last CTL it will
    // include some zeros.
    res.extend(
        cols.block_bytes[i * U8S_PER_CTL..]
            .chunks(size_of::<u32>())
            .map(|chunk| Column::le_bytes(chunk))
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    // The output contains the XOR'd rate part.
    res.extend(
        Column::singles(&cols.xored_rate_u32s[i * U32S_PER_CTL..])
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    res
}

/// CTL filter for the final block rows of the `KeccakSponge` table.
pub(crate) fn ctl_looked_filter<F: Field>() -> Filter<F> {
    // The CPU table is only interested in our final-block rows, since those contain
    // the final sponge output.
    Filter::new_simple(Column::sum(KECCAK_SPONGE_COL_MAP.is_final_input_len))
}

/// CTL filter for reading the `i`th byte of input from memory.
pub(crate) fn ctl_looking_memory_filter<F: Field>(i: usize) -> Filter<F> {
    // We perform the `i`th read if either
    // - this is a full input block, or
    // - this is a final block of length `i` or greater
    let cols = KECCAK_SPONGE_COL_MAP;
    if i == KECCAK_RATE_BYTES - 1 {
        Filter::new_simple(Column::single(cols.is_full_input_block))
    } else {
        Filter::new_simple(Column::sum(
            once(&cols.is_full_input_block).chain(&cols.is_final_input_len[i + 1..]),
        ))
    }
}

/// CTL filter for looking at XORs in the logic table.
pub(crate) fn ctl_looking_logic_filter<F: Field>() -> Filter<F> {
    let cols = KECCAK_SPONGE_COL_MAP;
    Filter::new_simple(Column::sum(
        once(&cols.is_full_input_block).chain(&cols.is_final_input_len),
    ))
}

/// CTL filter for looking at the input and output in the Keccak table.
pub(crate) fn ctl_looking_keccak_filter<F: Field>() -> Filter<F> {
    let cols = KECCAK_SPONGE_COL_MAP;
    Filter::new_simple(Column::sum(
        once(&cols.is_full_input_block).chain(&cols.is_final_input_len),
    ))
}
*/

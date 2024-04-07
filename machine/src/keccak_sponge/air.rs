use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_matrix::MatrixRowSlices;

use super::columns::{
    KeccakSpongeCols, KECCAK_DIGEST_U16S, KECCAK_RATE_BYTES, KECCAK_RATE_U16S,
    NUM_KECCAK_SPONGE_COLS,
};
use super::{KeccakSpongeChip};

impl<F> BaseAir<F> for KeccakSpongeChip {
    fn width(&self) -> usize {
        NUM_KECCAK_SPONGE_COLS
    }
}

impl<AB: AirBuilder> Air<AB> for KeccakSpongeChip {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &KeccakSpongeCols<AB::Var> = main.row_slice(0).borrow();
        let next: &KeccakSpongeCols<AB::Var> = main.row_slice(1).borrow();

        // // Check the range column: First value must be 0, last row
        // // must be 255, and intermediate rows must increment by 0
        // // or 1.
        // let rc1 = local.range_counter;
        // let rc2 = next.range_counter;
        // builder.when_first_row().assert_zero(rc1);
        // let incr = rc2 - rc1;
        // builder.when_transition().assert_bool(incr);
        // let range_max = AB::Expr::from_canonical_u64((BYTE_RANGE_MAX - 1) as u64);
        // builder.when_last_row().assert_eq(rc1, range_max);

        // Each flag (full-input block, final block or implied dummy flag) must be
        // boolean.
        let is_full_input_block = local.is_full_input_block;
        builder.assert_bool(is_full_input_block);

        let is_final_block = local
            .is_final_input_len
            .iter()
            .copied()
            .fold(AB::Expr::zero(), |acc, is_final_len| acc + is_final_len);
        builder.assert_bool(is_final_block.clone());

        for &is_final_len in local.is_final_input_len.iter() {
            builder.assert_bool(is_final_len);
        }

        // Ensure that full-input block and final block flags are not set to 1 at the
        // same time.
        builder.assert_zero(is_final_block.clone() * is_full_input_block);

        // If this is the first row, the original sponge state should be 0 and
        // already_absorbed_bytes = 0.
        let already_absorbed_bytes = local.already_absorbed_bytes;
        builder.when_first_row().assert_zero(already_absorbed_bytes);
        for &original_rate_elem in local.original_rate_u16s.iter() {
            builder.when_first_row().assert_zero(original_rate_elem);
        }
        for &original_capacity_elem in local.original_capacity_u16s.iter() {
            builder.when_first_row().assert_zero(original_capacity_elem);
        }

        // If this is a final block, the next row's original sponge state should be 0
        // and already_absorbed_bytes = 0.
        builder
            .when(is_final_block.clone())
            .assert_zero(next.already_absorbed_bytes);
        for &original_rate_elem in next.original_rate_u16s.iter() {
            builder
                .when(is_final_block.clone())
                .assert_zero(original_rate_elem);
        }
        for &original_capacity_elem in next.original_capacity_u16s.iter() {
            builder
                .when(is_final_block.clone())
                .assert_zero(original_capacity_elem);
        }

        // If this is a full-input block, the next row's "before" should match our
        // "after" state.
        for (current_bytes_after, &next_before) in local
            .updated_digest_state_bytes
            .chunks_exact(2)
            .zip(&next.original_rate_u16s[..KECCAK_DIGEST_U16S])
        {
            let current_after = (0..2).fold(AB::Expr::zero(), |acc, i| {
                acc + current_bytes_after[i] * AB::Expr::from_canonical_usize(1 << (8 * i))
            });
            builder
                .when(is_full_input_block)
                .assert_zero(next_before - current_after);
        }
        for (&current_after, &next_before) in local
            .partial_updated_state_u16s
            .iter()
            .zip(next.original_rate_u16s[KECCAK_DIGEST_U16S..].iter())
        {
            builder
                .when(is_full_input_block)
                .assert_zero(next_before - current_after);
        }
        for (&current_after, &next_before) in local
            .partial_updated_state_u16s
            .iter()
            .skip(KECCAK_RATE_U16S - KECCAK_DIGEST_U16S)
            .zip(next.original_capacity_u16s.iter())
        {
            builder
                .when(is_full_input_block)
                .assert_zero(next_before - current_after);
        }

        // If this is a full-input block, the next row's already_absorbed_bytes should
        // be ours plus `KECCAK_RATE_BYTES`.
        builder.when(is_full_input_block).assert_zero(
            already_absorbed_bytes + AB::Expr::from_canonical_usize(KECCAK_RATE_BYTES)
                - next.already_absorbed_bytes,
        );

        // // A dummy row is always followed by another dummy row, so the prover can't put
        // // dummy rows "in between" to avoid the above checks.
        // let is_dummy = AB::Expr::one() - is_full_input_block - is_final_block;
        // let next_is_final_block = next
        //     .is_final_input_len
        //     .iter()
        //     .fold(AB::Expr::zero(), |acc, &is_final_len| acc + is_final_len);
        // builder
        //     .when(is_dummy)
        //     .assert_zero(next.is_full_input_block + next_is_final_block);
    }
}

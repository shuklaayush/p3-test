use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_matrix::Matrix;

use super::{
    columns::{MemoryCols, NUM_MEMORY_COLS},
    MemoryChip,
};

impl<F> BaseAir<F> for MemoryChip {
    fn width(&self) -> usize {
        NUM_MEMORY_COLS
    }
}

impl<AB: AirBuilder> Air<AB> for MemoryChip {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let next = main.row_slice(1);
        let local: &MemoryCols<AB::Var> = (*local).borrow();
        let next: &MemoryCols<AB::Var> = (*next).borrow();

        // TODO: Add more constraints.
        builder.assert_bool(local.is_read);
        builder.assert_bool(local.is_write);

        builder.assert_zero(local.is_read * local.is_write);

        builder.assert_bool(local.addr_unchanged);

        builder
            .when_transition()
            .when(local.addr_unchanged)
            .assert_eq(local.addr, next.addr);

        let diff = next.diff_limb_lo
            + next.diff_limb_md * AB::Expr::from_canonical_u32(1 << 8)
            + next.diff_limb_hi * AB::Expr::from_canonical_u32(1 << 16);
        builder
            .when_transition()
            .when(next.addr_unchanged)
            .assert_eq(diff.clone(), next.timestamp - local.timestamp);
        builder
            .when_transition()
            .when(next.is_read + next.is_write)
            .when_ne(next.addr_unchanged, AB::Expr::one())
            .assert_eq(diff, next.addr - local.addr - AB::Expr::one());

        // TODO: Do I need this?
        builder
            .when_transition()
            .when(next.addr_unchanged)
            .when(next.is_read)
            .assert_eq(local.value, next.value);

        // TODO: Check memory is initialized properly
        // builder
        //     .when_ne(local.addr_unchanged, AB::Expr::one())
        //     .when(local.is_read)
        //     .assert_zero(local.value);
    }
}

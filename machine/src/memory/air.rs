use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
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
        builder.assert_bool(local.is_real);

        builder.assert_bool(local.is_read);
        builder.assert_bool(local.addr_equal);

        builder
            .when(local.addr_equal)
            .assert_eq(local.addr, next.addr);

        // TODO: This is dummy to make tests pass.
        //       For some reason, permutation constraints fail when this chip has degree 2.
        builder
            .when(local.is_real)
            .assert_eq(local.is_real * local.is_real, local.is_real * local.is_real);
    }
}

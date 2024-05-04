use core::mem::{size_of, transmute};
use p3_test_derive::AlignedBorrow;
use p3_util::indices_arr;

#[cfg(feature = "debug-trace")]
use p3_test_derive::Headers;

#[repr(C)]
#[derive(AlignedBorrow)]
#[cfg_attr(feature = "debug-trace", derive(Headers))]
pub struct MemoryCols<T> {
    pub is_real: T,

    pub addr: T,

    pub timestamp: T,

    pub value: T,

    pub is_read: T,

    pub addr_equal: T,

    /// Either addr' - addr - 1 (if address changed), or timestamp' - timestamp (if address is not changed)
    pub diff: T,
}

impl<T: Copy> MemoryCols<T> {}

pub(crate) const NUM_MEMORY_COLS: usize = size_of::<MemoryCols<u8>>();
pub(crate) const MEMORY_COL_MAP: MemoryCols<usize> = make_col_map();

const fn make_col_map() -> MemoryCols<usize> {
    let indices_arr = indices_arr::<NUM_MEMORY_COLS>();
    unsafe { transmute::<[usize; NUM_MEMORY_COLS], MemoryCols<usize>>(indices_arr) }
}

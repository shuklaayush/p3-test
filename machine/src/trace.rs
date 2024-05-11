use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{Domain, StarkGenericConfig, Val};

pub struct ChipTrace<SC: StarkGenericConfig> {
    pub matrix: RowMajorMatrix<Val<SC>>,
    pub domain: Domain<SC>,
    pub opening_index: usize,
}

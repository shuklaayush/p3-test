use p3_air::VirtualPairCol;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;

pub struct Interaction<F: Field> {
    pub fields: Vec<VirtualPairCol<F>>,
    pub count: VirtualPairCol<F>,
    pub argument_index: usize,
}

pub trait Chip<F: Field> {
    fn generate_trace(&self) -> RowMajorMatrix<F>;

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![]
    }
}

use p3_commit::PolynomialSpace;
use p3_field::{ExtensionField, Field};
use p3_matrix::dense::RowMajorMatrix;

#[derive(Clone)]
pub struct ChipTrace<F, Domain>
where
    F: Field,
    Domain: PolynomialSpace,
{
    pub matrix: RowMajorMatrix<F>,
    pub domain: Domain,
    pub opening_index: usize,
}

impl<EF, Domain> ChipTrace<EF, Domain>
where
    EF: Field,
    Domain: PolynomialSpace,
{
    pub fn flatten_to_base<F: Field>(&self) -> ChipTrace<F, Domain>
    where
        EF: ExtensionField<F>,
    {
        ChipTrace {
            matrix: self.matrix.flatten_to_base(),
            domain: self.domain,
            opening_index: self.opening_index,
        }
    }
}

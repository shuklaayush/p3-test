use alloc::vec::Vec;
use core::iter::Iterator;
use core::marker::PhantomData;
use core::ops::Deref;
use core::slice::Iter;

use p3_air::{
    AirBuilder, AirBuilderWithPublicValues, ExtensionBuilder, PairBuilder, PermutationAirBuilder,
};
use p3_matrix::Matrix;

/// A subset of a matrix. The matrix will contain a subset of the elements of `self.inner`.
pub struct SubMatrix<T, M>
where
    T: Send + Sync,
    M: Matrix<T>,
{
    inner: M,
    indices: Vec<usize>,
    _phantom: PhantomData<T>,
}

impl<T, M> SubMatrix<T, M>
where
    T: Send + Sync,
    M: Matrix<T>,
{
    pub const fn new(inner: M, indices: Vec<usize>) -> Self {
        Self {
            inner,
            indices,
            _phantom: PhantomData,
        }
    }
}

/// An iterator that maps over matrix elements based on a set of indices.
pub struct RowIterator<'a, T, M>
where
    T: Send + Sync + 'a,
    M: Matrix<T> + 'a,
{
    inner: &'a M,
    row: usize,
    indices: Iter<'a, usize>,
    _phantom: PhantomData<T>,
}

impl<'a, T, M> Iterator for RowIterator<'a, T, M>
where
    T: Send + Sync,
    M: Matrix<T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.indices.next().map(|&i| self.inner.get(self.row, i))
    }
}

/// Implement `Matrix` for `SubMatrix`.
impl<T, M> Matrix<T> for SubMatrix<T, M>
where
    T: Send + Sync,
    M: Matrix<T>,
{
    type Row<'a> = RowIterator<'a, T, M> where Self: 'a;

    #[inline]
    fn row(&self, r: usize) -> Self::Row<'_> {
        RowIterator {
            inner: &self.inner,
            row: r,
            indices: self.indices.iter(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn row_slice(&self, r: usize) -> impl Deref<Target = [T]> {
        self.row(r).collect::<Vec<_>>()
    }

    #[inline]
    fn width(&self) -> usize {
        self.indices.len()
    }

    #[inline]
    fn height(&self) -> usize {
        self.inner.height()
    }
}

/// A builder used to eval a sub-air. This will handle enforcing constraints for a subset of elements
/// of a trace matrix. E.g., if a particular air needs to be enforced for a subset of the elements
/// of the trace, then the SubsetAirBuilder can be used.
pub struct SubsetAirBuilder<'a, AB: AirBuilder> {
    inner: &'a mut AB,
    preprocessed_indices: Vec<usize>,
    main_indices: Vec<usize>,
}

impl<'a, AB: AirBuilder> SubsetAirBuilder<'a, AB> {
    pub fn new(
        inner: &'a mut AB,
        preprocessed_indices: Vec<usize>,
        main_indices: Vec<usize>,
    ) -> Self {
        Self {
            inner,
            preprocessed_indices,
            main_indices,
        }
    }
}

impl<'a, AB: AirBuilder> AirBuilder for SubsetAirBuilder<'a, AB> {
    type F = AB::F;
    type Expr = AB::Expr;
    type Var = AB::Var;
    type M = SubMatrix<Self::Var, AB::M>;

    fn main(&self) -> Self::M {
        let matrix = self.inner.main();
        SubMatrix::new(matrix, self.main_indices.clone())
    }

    fn is_first_row(&self) -> Self::Expr {
        self.inner.is_first_row()
    }

    fn is_last_row(&self) -> Self::Expr {
        self.inner.is_last_row()
    }

    fn is_transition_window(&self, size: usize) -> Self::Expr {
        self.inner.is_transition_window(size)
    }

    fn assert_zero<I: Into<Self::Expr>>(&mut self, x: I) {
        self.inner.assert_zero(x.into());
    }
}

impl<'a, AB: PairBuilder> PairBuilder for SubsetAirBuilder<'a, AB> {
    fn preprocessed(&self) -> Self::M {
        let matrix = self.inner.main();
        SubMatrix::new(matrix, self.preprocessed_indices.clone())
    }
}

impl<'a, AB: AirBuilderWithPublicValues> AirBuilderWithPublicValues for SubsetAirBuilder<'a, AB> {
    type PublicVar = AB::PublicVar;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.inner.public_values()
    }
}

impl<'a, AB: ExtensionBuilder> ExtensionBuilder for SubsetAirBuilder<'a, AB> {
    type EF = AB::EF;
    type ExprEF = AB::ExprEF;
    type VarEF = AB::VarEF;

    fn assert_zero_ext<I>(&mut self, x: I)
    where
        I: Into<Self::ExprEF>,
    {
        self.inner.assert_zero_ext(x.into());
    }
}

impl<'a, AB: PermutationAirBuilder> PermutationAirBuilder for SubsetAirBuilder<'a, AB> {
    type MP = AB::MP;

    type RandomVar = AB::RandomVar;

    fn permutation(&self) -> Self::MP {
        self.inner.permutation()
    }

    fn permutation_randomness(&self) -> &[Self::RandomVar] {
        self.inner.permutation_randomness()
    }
}

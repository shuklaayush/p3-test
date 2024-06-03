use alloc::vec::Vec;
use core::iter::{Skip, Take};
use core::marker::PhantomData;
use core::ops::{Deref, Range};

use p3_air::{
    AirBuilder, AirBuilderWithPublicValues, ExtensionBuilder, PairBuilder, PermutationAirBuilder,
};
use p3_matrix::Matrix;

/// A submatrix of a matrix. The matrix will contain a subset of the columns of `self.inner`.
pub struct SubMatrixRange<T, M>
where
    T: Send + Sync,
    M: Matrix<T>,
{
    inner: M,
    column_range: Range<usize>,
    _phantom: PhantomData<T>,
}

impl<T, M> SubMatrixRange<T, M>
where
    T: Send + Sync,
    M: Matrix<T>,
{
    pub const fn new(inner: M, column_range: Range<usize>) -> Self {
        Self {
            inner,
            column_range,
            _phantom: PhantomData,
        }
    }
}

/// Implement `Matrix` for `SubMatrixRange`.
impl<T, M> Matrix<T> for SubMatrixRange<T, M>
where
    T: Send + Sync,
    M: Matrix<T>,
{
    type Row<'a> = Skip<Take<M::Row<'a>>> where Self: 'a;

    #[inline]
    fn row(&self, r: usize) -> Self::Row<'_> {
        self.inner
            .row(r)
            .take(self.column_range.end)
            .skip(self.column_range.start)
    }

    #[inline]
    fn row_slice(&self, r: usize) -> impl Deref<Target = [T]> {
        self.row(r).collect::<Vec<_>>()
    }

    #[inline]
    fn width(&self) -> usize {
        self.column_range.len()
    }

    #[inline]
    fn height(&self) -> usize {
        self.inner.height()
    }
}

/// A builder used to eval a sub-air.  This will handle enforcing constraints for a subset of a
/// trace matrix.  E.g. if a particular air needs to be enforced for a subset of the columns of
/// the trace, then the SubRangeAirBuilder can be used.
pub struct SubRangeAirBuilder<'a, AB: AirBuilder> {
    inner: &'a mut AB,
    main_range: Range<usize>,
    preprocessed_range: Range<usize>,
}

impl<'a, AB: AirBuilder> SubRangeAirBuilder<'a, AB> {
    pub fn new(
        inner: &'a mut AB,
        preprocessed_range: Range<usize>,
        main_range: Range<usize>,
    ) -> Self {
        Self {
            inner,
            preprocessed_range,
            main_range,
        }
    }

    pub fn new_main(inner: &'a mut AB, main_range: Range<usize>) -> Self {
        Self::new(inner, 0..0, main_range)
    }

    pub fn new_preprocessed(inner: &'a mut AB, preprocessed_range: Range<usize>) -> Self {
        Self::new(inner, preprocessed_range, 0..0)
    }
}

impl<'a, AB: AirBuilder> AirBuilder for SubRangeAirBuilder<'a, AB> {
    type F = AB::F;
    type Expr = AB::Expr;
    type Var = AB::Var;
    type M = SubMatrixRange<Self::Var, AB::M>;

    fn main(&self) -> Self::M {
        let matrix = self.inner.main();
        SubMatrixRange::new(matrix, self.main_range.clone())
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

impl<'a, AB: PairBuilder> PairBuilder for SubRangeAirBuilder<'a, AB> {
    fn preprocessed(&self) -> Self::M {
        let matrix = self.inner.main();
        SubMatrixRange::new(matrix, self.preprocessed_range.clone())
    }
}

impl<'a, AB: AirBuilderWithPublicValues> AirBuilderWithPublicValues for SubRangeAirBuilder<'a, AB> {
    type PublicVar = AB::PublicVar;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.inner.public_values()
    }
}

impl<'a, AB: ExtensionBuilder> ExtensionBuilder for SubRangeAirBuilder<'a, AB> {
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

impl<'a, AB: PermutationAirBuilder> PermutationAirBuilder for SubRangeAirBuilder<'a, AB> {
    type MP = AB::MP;

    type RandomVar = AB::RandomVar;

    fn permutation(&self) -> Self::MP {
        self.inner.permutation()
    }

    fn permutation_randomness(&self) -> &[Self::RandomVar] {
        self.inner.permutation_randomness()
    }
}

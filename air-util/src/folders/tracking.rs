use p3_air::{AirBuilder, AirBuilderWithPublicValues, PairBuilder};
use p3_field::{Field, PrimeField32};

use super::ViewPair;
use crate::util::tracked_field::TrackedField;

pub struct TrackingConstraintBuilder<'a, F, const SET_SIZE: usize>
where
    F: Field,
{
    pub row_index: usize,
    pub col_indices: Vec<usize>,
    pub preprocessed: ViewPair<'a, TrackedField<F, SET_SIZE>>,
    pub main: ViewPair<'a, TrackedField<F, SET_SIZE>>,
    pub public_values: &'a [TrackedField<F, SET_SIZE>],
    pub is_first_row: F,
    pub is_last_row: F,
    pub is_transition: F,
}

impl<'a, F, const SET_SIZE: usize> AirBuilder for TrackingConstraintBuilder<'a, F, SET_SIZE>
where
    F: Field,
{
    type F = TrackedField<F, SET_SIZE>;
    type Expr = TrackedField<F, SET_SIZE>;
    type Var = TrackedField<F, SET_SIZE>;
    type M = ViewPair<'a, TrackedField<F, SET_SIZE>>;

    fn main(&self) -> Self::M {
        self.main
    }

    fn is_first_row(&self) -> Self::Expr {
        self.is_first_row.into()
    }

    fn is_last_row(&self) -> Self::Expr {
        self.is_last_row.into()
    }

    fn is_transition_window(&self, size: usize) -> Self::Expr {
        if size == 2 {
            self.is_transition.into()
        } else {
            panic!("only supports a window size of 2")
        }
    }

    fn assert_zero<I: Into<Self::Expr>>(&mut self, x: I) {
        let x = x.into();
        if x != F::zero().into() {
            let indices = x.origin.iter().collect::<Vec<_>>();
            self.col_indices.extend(indices);
        }
    }

    fn assert_eq<I1: Into<Self::Expr>, I2: Into<Self::Expr>>(&mut self, x: I1, y: I2) {
        let x = x.into();
        let y = y.into();
        if x != y {
            let indices = x.origin.iter().chain(y.origin.iter()).collect::<Vec<_>>();
            self.col_indices.extend(indices);
        }
    }
}

impl<'a, F, const SET_SIZE: usize> PairBuilder for TrackingConstraintBuilder<'a, F, SET_SIZE>
where
    F: PrimeField32,
{
    fn preprocessed(&self) -> Self::M {
        self.preprocessed
    }
}

impl<'a, F, const SET_SIZE: usize> AirBuilderWithPublicValues
    for TrackingConstraintBuilder<'a, F, SET_SIZE>
where
    F: PrimeField32,
{
    type PublicVar = TrackedField<F, SET_SIZE>;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.public_values
    }
}

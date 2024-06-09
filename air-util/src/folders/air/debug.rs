use p3_air::{AirBuilder, AirBuilderWithPublicValues, PairBuilder};
use p3_field::Field;

use crate::folders::ViewPair;

/// An `AirBuilder` which asserts that each constraint is zero, allowing any failed constraints to
/// be detected early.
pub struct DebugConstraintBuilder<'a, F: Field> {
    pub row_index: usize,
    pub preprocessed: ViewPair<'a, F>,
    pub main: ViewPair<'a, F>,
    pub public_values: &'a [F],
    pub is_first_row: F,
    pub is_last_row: F,
    pub is_transition: F,
}

impl<'a, F: Field> AirBuilder for DebugConstraintBuilder<'a, F> {
    type F = F;
    type Expr = F;
    type Var = F;
    type M = ViewPair<'a, F>;

    fn main(&self) -> Self::M {
        self.main
    }

    fn is_first_row(&self) -> Self::Expr {
        self.is_first_row
    }

    fn is_last_row(&self) -> Self::Expr {
        self.is_last_row
    }

    fn is_transition_window(&self, size: usize) -> Self::Expr {
        if size == 2 {
            self.is_transition
        } else {
            panic!("only supports a window size of 2")
        }
    }

    fn assert_zero<I: Into<Self::Expr>>(&mut self, x: I) {
        assert!(
            x.into().is_zero(),
            "constraints had nonzero value on row {}",
            self.row_index
        );
    }

    fn assert_eq<I1: Into<Self::Expr>, I2: Into<Self::Expr>>(&mut self, x: I1, y: I2) {
        let x = x.into();
        let y = y.into();
        assert_eq!(
            x, y,
            "values didn't match on row {}: {} != {}",
            self.row_index, x, y
        );
    }
}

impl<'a, F: Field> PairBuilder for DebugConstraintBuilder<'a, F> {
    fn preprocessed(&self) -> Self::M {
        self.preprocessed
    }
}

impl<'a, F: Field> AirBuilderWithPublicValues for DebugConstraintBuilder<'a, F> {
    type PublicVar = F;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.public_values
    }
}

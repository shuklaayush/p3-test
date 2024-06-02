use p3_air::{
    AirBuilder, AirBuilderWithPublicValues, ExtensionBuilder, PairBuilder, PermutationAirBuilder,
};
use p3_field::{ExtensionField, Field};
use p3_interaction::{InteractionAirBuilder, NUM_PERM_CHALLENGES};

use super::ViewPair;

/// An `AirBuilder` which asserts that each constraint is zero, allowing any failed constraints to
/// be detected early.
pub struct DebugConstraintBuilder<'a, F: Field, EF: ExtensionField<F>> {
    pub row_index: usize,
    pub preprocessed: ViewPair<'a, F>,
    pub main: ViewPair<'a, F>,
    pub permutation: ViewPair<'a, EF>,
    pub perm_challenges: [EF; NUM_PERM_CHALLENGES],
    pub public_values: &'a [F],
    pub cumulative_sum: EF,
    pub is_first_row: F,
    pub is_last_row: F,
    pub is_transition: F,
}

impl<'a, F: Field, EF: ExtensionField<F>> AirBuilder for DebugConstraintBuilder<'a, F, EF> {
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

impl<'a, F: Field, EF: ExtensionField<F>> PairBuilder for DebugConstraintBuilder<'a, F, EF> {
    fn preprocessed(&self) -> Self::M {
        self.preprocessed
    }
}

impl<'a, F: Field, EF: ExtensionField<F>> AirBuilderWithPublicValues
    for DebugConstraintBuilder<'a, F, EF>
{
    type PublicVar = F;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.public_values
    }
}

impl<'a, F: Field, EF: ExtensionField<F>> ExtensionBuilder for DebugConstraintBuilder<'a, F, EF> {
    type EF = EF;
    type ExprEF = EF;
    type VarEF = EF;

    fn assert_zero_ext<I>(&mut self, x: I)
    where
        I: Into<Self::ExprEF>,
    {
        assert!(
            x.into().is_zero(),
            "constraints had nonzero value on row {}",
            self.row_index
        );
    }

    fn assert_eq_ext<I1, I2>(&mut self, x: I1, y: I2)
    where
        I1: Into<Self::ExprEF>,
        I2: Into<Self::ExprEF>,
    {
        let x = x.into();
        let y = y.into();
        assert_eq!(
            x, y,
            "values didn't match on row {}: {} != {}",
            self.row_index, x, y
        );
    }
}

impl<'a, F: Field, EF: ExtensionField<F>> PermutationAirBuilder
    for DebugConstraintBuilder<'a, F, EF>
{
    type MP = ViewPair<'a, EF>;

    type RandomVar = EF;

    fn permutation(&self) -> Self::MP {
        self.permutation
    }

    fn permutation_randomness(&self) -> &[Self::EF] {
        &self.perm_challenges
    }
}

impl<'a, F: Field, EF: ExtensionField<F>> InteractionAirBuilder
    for DebugConstraintBuilder<'a, F, EF>
{
    fn cumulative_sum(&self) -> Self::RandomVar {
        self.cumulative_sum
    }
}

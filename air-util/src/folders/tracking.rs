use alloc::collections::BTreeSet;

use p3_air::{
    AirBuilder, AirBuilderWithPublicValues, ExtensionBuilder, PairBuilder, PermutationAirBuilder,
};
use p3_field::{ExtensionField, Field};
use p3_interaction::{InteractionAirBuilder, NUM_PERM_CHALLENGES};

use super::ViewPair;
use crate::util::{
    TraceEntry, TrackedExtensionFieldExpression, TrackedFieldExpression, TrackedFieldVariable,
};

// TODO: Remove permutations?
pub struct TrackingConstraintBuilder<'a, F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    pub entries: BTreeSet<TraceEntry>,
    pub preprocessed: ViewPair<'a, TrackedFieldVariable<F, TraceEntry>>,
    pub main: ViewPair<'a, TrackedFieldVariable<F, TraceEntry>>,
    pub permutation: ViewPair<'a, TrackedFieldVariable<EF, TraceEntry>>,
    pub perm_challenges: [TrackedFieldVariable<EF, TraceEntry>; NUM_PERM_CHALLENGES],
    pub public_values: &'a [TrackedFieldVariable<F, TraceEntry>],
    pub cumulative_sum: TrackedFieldVariable<EF, TraceEntry>,
    pub is_first_row: F,
    pub is_last_row: F,
    pub is_transition: F,
}

impl<'a, F, EF> AirBuilder for TrackingConstraintBuilder<'a, F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    type F = F;
    type Expr = TrackedFieldExpression<F, TraceEntry>;
    type Var = TrackedFieldVariable<F, TraceEntry>;
    type M = ViewPair<'a, TrackedFieldVariable<F, TraceEntry>>;

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
        if !x.value.is_zero() {
            self.entries.extend(x.origin);
        }
    }
}

impl<'a, F, EF> PairBuilder for TrackingConstraintBuilder<'a, F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    fn preprocessed(&self) -> Self::M {
        self.preprocessed
    }
}

impl<'a, F, EF> AirBuilderWithPublicValues for TrackingConstraintBuilder<'a, F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    type PublicVar = TrackedFieldVariable<F, TraceEntry>;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.public_values
    }
}

impl<'a, F, EF> ExtensionBuilder for TrackingConstraintBuilder<'a, F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    type EF = EF;
    type ExprEF = TrackedExtensionFieldExpression<F, EF, TraceEntry>;
    type VarEF = TrackedFieldVariable<EF, TraceEntry>;

    fn assert_zero_ext<I>(&mut self, x: I)
    where
        I: Into<Self::ExprEF>,
    {
        let x = x.into();
        if !x.0.value.is_zero() {
            self.entries.extend(x.0.origin);
        }
    }
}

impl<'a, F, EF> PermutationAirBuilder for TrackingConstraintBuilder<'a, F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    type MP = ViewPair<'a, Self::VarEF>;
    type RandomVar = Self::VarEF;

    fn permutation(&self) -> Self::MP {
        self.permutation
    }

    fn permutation_randomness(&self) -> &[Self::RandomVar] {
        &self.perm_challenges
    }
}

impl<'a, F, EF> InteractionAirBuilder for TrackingConstraintBuilder<'a, F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    fn cumulative_sum(&self) -> Self::VarEF {
        self.cumulative_sum
    }
}

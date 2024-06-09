use p3_air::{AirBuilder, AirBuilderWithPublicValues, PairBuilder};
use p3_field::Field;

use crate::folders::{EntriesLog, ViewPair};
use crate::util::{TraceEntry, TrackedFieldExpression, TrackedFieldVariable};

pub struct TrackingConstraintBuilder<'a, F>
where
    F: Field,
{
    pub entries: EntriesLog<TraceEntry>,
    pub preprocessed: ViewPair<'a, TrackedFieldVariable<F, TraceEntry>>,
    pub main: ViewPair<'a, TrackedFieldVariable<F, TraceEntry>>,
    pub public_values: &'a [TrackedFieldVariable<F, TraceEntry>],
    pub is_first_row: F,
    pub is_last_row: F,
    pub is_transition: F,
}

impl<'a, F> AirBuilder for TrackingConstraintBuilder<'a, F>
where
    F: Field,
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
        self.entries.constrained.extend(x.constraint_origin);
        if !x.value.is_zero() {
            self.entries.failing.extend(x.value_origin);
        }
    }
}

impl<'a, F> PairBuilder for TrackingConstraintBuilder<'a, F>
where
    F: Field,
{
    fn preprocessed(&self) -> Self::M {
        self.preprocessed
    }
}

impl<'a, F> AirBuilderWithPublicValues for TrackingConstraintBuilder<'a, F>
where
    F: Field,
{
    type PublicVar = TrackedFieldVariable<F, TraceEntry>;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.public_values
    }
}

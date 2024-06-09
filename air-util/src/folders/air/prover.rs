use p3_air::{AirBuilder, AirBuilderWithPublicValues, PairBuilder};
use p3_uni_stark::{PackedChallenge, PackedVal, StarkGenericConfig, Val};

use crate::folders::ViewPair;

/// A folder for prover constraints.
pub struct ProverConstraintFolder<'a, SC: StarkGenericConfig> {
    pub preprocessed: ViewPair<'a, PackedVal<SC>>,
    pub main: ViewPair<'a, PackedVal<SC>>,
    pub public_values: &'a [Val<SC>],
    pub is_first_row: PackedVal<SC>,
    pub is_last_row: PackedVal<SC>,
    pub is_transition: PackedVal<SC>,
    pub alpha: PackedChallenge<SC>,
    pub accumulator: PackedChallenge<SC>,
}

impl<'a, SC> AirBuilder for ProverConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    type F = Val<SC>;
    type Expr = PackedVal<SC>;
    type Var = PackedVal<SC>;
    type M = ViewPair<'a, PackedVal<SC>>;

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
        let x: PackedVal<SC> = x.into();
        self.accumulator *= self.alpha;
        self.accumulator += x;
    }
}

impl<'a, SC> PairBuilder for ProverConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn preprocessed(&self) -> Self::M {
        self.preprocessed
    }
}

impl<'a, SC: StarkGenericConfig> AirBuilderWithPublicValues for ProverConstraintFolder<'a, SC> {
    type PublicVar = Self::F;

    fn public_values(&self) -> &[Self::PublicVar] {
        self.public_values
    }
}

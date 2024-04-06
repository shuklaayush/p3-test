// Folder: Folding builder
use p3_air::{
    AirBuilder, AirBuilderWithPublicValues, ExtensionBuilder, PairBuilder, PermutationAirBuilder,
    TwoRowMatrixView,
};
use p3_field::AbstractField;
use p3_uni_stark::{PackedChallenge, PackedVal, StarkGenericConfig, Val};

/// A folder for prover constraints.
pub struct ProverConstraintFolder<'a, SC: StarkGenericConfig> {
    pub preprocessed: TwoRowMatrixView<'a, PackedVal<SC>>,
    pub main: TwoRowMatrixView<'a, PackedVal<SC>>,
    pub public_values: &'a Vec<Val<SC>>,
    pub perm: TwoRowMatrixView<'a, PackedChallenge<SC>>,
    pub perm_challenges: &'a [SC::Challenge],
    pub cumulative_sum: SC::Challenge,
    pub is_first_row: PackedVal<SC>,
    pub is_last_row: PackedVal<SC>,
    pub is_transition: PackedVal<SC>,
    pub alpha: SC::Challenge,
    pub accumulator: PackedChallenge<SC>,
}

pub struct VerifierConstraintFolder<'a, SC: StarkGenericConfig> {
    pub preprocessed: TwoRowMatrixView<'a, SC::Challenge>,
    pub main: TwoRowMatrixView<'a, SC::Challenge>,
    pub public_values: &'a Vec<Val<SC>>,
    pub perm: TwoRowMatrixView<'a, SC::Challenge>,
    pub perm_challenges: &'a [SC::Challenge],
    pub is_first_row: SC::Challenge,
    pub is_last_row: SC::Challenge,
    pub is_transition: SC::Challenge,
    pub alpha: SC::Challenge,
    pub accumulator: SC::Challenge,
}

impl<'a, SC> AirBuilder for ProverConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    type F = Val<SC>;
    type Expr = PackedVal<SC>;
    type Var = PackedVal<SC>;
    type M = TwoRowMatrixView<'a, PackedVal<SC>>;

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
        self.accumulator *= PackedChallenge::<SC>::from_f(self.alpha);
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

impl<'a, SC> ExtensionBuilder for ProverConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    type EF = SC::Challenge;
    type ExprEF = PackedChallenge<SC>;
    type VarEF = PackedChallenge<SC>;

    fn assert_zero_ext<I>(&mut self, x: I)
    where
        I: Into<Self::ExprEF>,
    {
        let x: PackedChallenge<SC> = x.into();
        self.accumulator *= PackedChallenge::<SC>::from_f(self.alpha);
        self.accumulator += x;
    }
}

impl<'a, SC> PermutationAirBuilder for ProverConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    type MP = TwoRowMatrixView<'a, PackedChallenge<SC>>;

    fn permutation(&self) -> Self::MP {
        self.perm
    }

    fn permutation_randomness(&self) -> &[Self::EF] {
        // TODO: implement
        self.perm_challenges
    }
}

impl<'a, SC: StarkGenericConfig> AirBuilderWithPublicValues for ProverConstraintFolder<'a, SC> {
    type PublicVar = Self::F;

    fn public_values(&self) -> &[Self::F] {
        self.public_values
    }
}

impl<'a, SC: StarkGenericConfig> AirBuilder for VerifierConstraintFolder<'a, SC> {
    type F = Val<SC>;
    type Expr = SC::Challenge;
    type Var = SC::Challenge;
    type M = TwoRowMatrixView<'a, SC::Challenge>;

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
            panic!("uni-stark only supports a window size of 2")
        }
    }

    fn assert_zero<I: Into<Self::Expr>>(&mut self, x: I) {
        let x: SC::Challenge = x.into();
        self.accumulator *= self.alpha;
        self.accumulator += x;
    }
}

impl<'a, SC> PairBuilder for VerifierConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn preprocessed(&self) -> Self::M {
        self.preprocessed
    }
}

impl<'a, SC> ExtensionBuilder for VerifierConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    type EF = SC::Challenge;
    type ExprEF = SC::Challenge;
    type VarEF = SC::Challenge;

    fn assert_zero_ext<I>(&mut self, x: I)
    where
        I: Into<Self::ExprEF>,
    {
        let x: SC::Challenge = x.into();
        self.accumulator *= SC::Challenge::from_f(self.alpha);
        self.accumulator += x;
    }
}

impl<'a, SC> PermutationAirBuilder for VerifierConstraintFolder<'a, SC>
where
    SC: StarkGenericConfig,
{
    type MP = TwoRowMatrixView<'a, SC::Challenge>;

    fn permutation(&self) -> Self::MP {
        self.perm
    }

    fn permutation_randomness(&self) -> &[Self::EF] {
        // TODO: implement
        self.perm_challenges
    }
}

impl<'a, SC: StarkGenericConfig> AirBuilderWithPublicValues for VerifierConstraintFolder<'a, SC> {
    type PublicVar = Self::F;

    fn public_values(&self) -> &[Self::F] {
        self.public_values
    }
}

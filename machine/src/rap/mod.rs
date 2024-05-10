pub mod interaction;
pub mod permutation_air;

use p3_air::{Air, PairBuilder};

use self::permutation_air::{PermutationAir, PermutationAirBuilderWithCumulativeSum};

pub trait Rap<AB: PermutationAirBuilderWithCumulativeSum + PairBuilder>:
    Air<AB> + PermutationAir<AB>
{
    fn preprocessed_width(&self) -> usize {
        0
    }

    fn eval_all(&self, builder: &mut AB) {
        self.eval(builder);
        self.eval_permutation_constraints(builder);
    }
}

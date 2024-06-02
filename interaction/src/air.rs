#[cfg(feature = "trace-writer")]
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;

use p3_air::{Air, ExtensionBuilder, PairBuilder, PermutationAirBuilder};
use p3_field::{AbstractField, Field};
use p3_matrix::Matrix;

use crate::interaction::{Interaction, InteractionType};
use crate::util::{generate_rlc_elements, reduce_row};

pub trait AirColumns {
    type ColumnMap;

    fn num_cols() -> usize;
    fn col_map() -> Self::ColumnMap;
    #[cfg(feature = "trace-writer")]
    fn headers() -> Vec<String>;
}

pub trait PairWithColumnTypes {
    type PreprocessedColumns: AirColumns;
    type MainColumns: AirColumns;

    fn preprocessed_width(&self) -> usize {
        Self::PreprocessedColumns::num_cols()
    }
}

pub trait InteractionAirBuilder: PermutationAirBuilder + PairBuilder {
    fn cumulative_sum(&self) -> Self::VarEF;
}

pub trait InteractionAir<F: Field> {
    fn receives(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn all_interactions(&self) -> Vec<(Interaction<F>, InteractionType)> {
        self.receives()
            .into_iter()
            .map(|i| (i, InteractionType::Receive))
            .chain(self.sends().into_iter().map(|i| (i, InteractionType::Send)))
            .collect()
    }
}

pub trait Rap<AB: InteractionAirBuilder>:
    Air<AB> + InteractionAir<AB::F> + PairWithColumnTypes
{
    fn permutation_width(&self) -> Option<usize> {
        let num_interactions = self.receives().len() + self.sends().len();
        if num_interactions > 0 {
            Some(num_interactions + 1)
        } else {
            None
        }
    }

    fn eval_permutation_constraints(&self, builder: &mut AB) {
        let interactions = self.all_interactions();
        if interactions.is_empty() {
            return;
        }

        let rand_elems = builder.permutation_randomness().to_vec();

        let main = builder.main();
        let (main_local, main_next) = (main.row_slice(0), main.row_slice(1));
        let main_local: &[AB::Var] = (*main_local).borrow();
        let main_next: &[AB::Var] = (*main_next).borrow();

        let preprocessed = builder.preprocessed();
        let preprocessed_local = preprocessed.row_slice(0);
        let preprocessed_next = preprocessed.row_slice(1);
        let preprocessed_local = (*preprocessed_local).borrow();
        let preprocessed_next = (*preprocessed_next).borrow();

        let perm = builder.permutation();
        let perm_local = perm.row_slice(0);
        let perm_next = perm.row_slice(1);
        let perm_local: &[AB::VarEF] = (*perm_local).borrow();
        let perm_next: &[AB::VarEF] = (*perm_next).borrow();
        let perm_width = perm.width();

        let phi_local = perm_local[perm_width - 1];
        let phi_next = perm_next[perm_width - 1];

        let alphas: Vec<AB::ExprEF> = generate_rlc_elements(&interactions, rand_elems[0].into());
        let betas = rand_elems[1].into().powers();

        let lhs = phi_next.into() - phi_local.into();
        let mut rhs = AB::ExprEF::zero();
        let mut phi_0 = AB::ExprEF::zero();
        for (m, (interaction, interaction_type)) in interactions.iter().enumerate() {
            // Reciprocal constraints
            let rlc = reduce_row(
                preprocessed_local,
                main_local,
                interaction.fields.as_slice(),
                alphas[interaction.argument_index].clone(),
                betas.clone(),
            );
            builder.assert_one_ext(rlc * perm_local[m].into());

            let mult_local = interaction
                .count
                .apply::<AB::Expr, AB::Var>(preprocessed_local, main_local);
            let mult_next = interaction
                .count
                .apply::<AB::Expr, AB::Var>(preprocessed_next, main_next);

            // Build the RHS of the permutation constraint
            match interaction_type {
                InteractionType::Send => {
                    phi_0 += perm_local[m].into() * mult_local;
                    rhs += perm_next[m].into() * mult_next;
                }
                InteractionType::Receive => {
                    phi_0 -= perm_local[m].into() * mult_local;
                    rhs -= perm_next[m].into() * mult_next;
                }
            }
        }

        let cumulative_sum = builder.cumulative_sum();

        // Running sum constraints
        builder.when_transition().assert_eq_ext(lhs, rhs);
        builder
            .when_first_row()
            .assert_eq_ext(*perm_local.last().unwrap(), phi_0);
        builder
            .when_last_row()
            .assert_eq_ext(*perm_local.last().unwrap(), cumulative_sum);
    }

    fn eval_all(&self, builder: &mut AB) {
        self.eval(builder);
        self.eval_permutation_constraints(builder);
    }
}

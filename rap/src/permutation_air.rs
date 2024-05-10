use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;

use p3_air::{ExtensionBuilder, PairBuilder, PermutationAirBuilder, VirtualPairCol};
use p3_field::{AbstractExtensionField, AbstractField, Powers};
use p3_matrix::Matrix;

use super::interaction::{Interaction, InteractionType};

pub trait PermutationAirBuilderWithCumulativeSum: PermutationAirBuilder {
    fn cumulative_sum(&self) -> Self::RandomVar;
}

pub fn generate_rlc_elements<F: AbstractField, EF: AbstractExtensionField<F>>(
    sends: &[Interaction<F>],
    receives: &[Interaction<F>],
    random_element: EF,
) -> Vec<EF> {
    random_element
        .powers()
        .skip(1)
        .take(
            sends
                .iter()
                .chain(receives)
                .map(|interaction| interaction.argument_index)
                .max()
                .unwrap_or(0)
                + 1,
        )
        .collect::<Vec<_>>()
}

pub fn reduce_row<Expr, Var, ExprEF>(
    preprocessed_row: &[Var],
    main_row: &[Var],
    fields: &[VirtualPairCol<Expr>],
    alpha: ExprEF,
    betas: Powers<ExprEF>,
) -> ExprEF
where
    Expr: AbstractField,
    Var: Into<Expr> + Copy,
    ExprEF: AbstractExtensionField<Expr>,
{
    let mut rlc = ExprEF::zero();
    for (columns, beta) in fields.iter().zip(betas) {
        rlc += beta * columns.apply::<Expr, Var>(preprocessed_row, main_row)
    }
    rlc += alpha;
    rlc
}

pub trait PermutationAir<AB: PermutationAirBuilderWithCumulativeSum + PairBuilder> {
    fn sends(&self) -> Vec<Interaction<AB::Expr>> {
        vec![]
    }

    fn receives(&self) -> Vec<Interaction<AB::Expr>> {
        vec![]
    }

    fn all_interactions(&self) -> Vec<(Interaction<AB::Expr>, InteractionType)> {
        self.sends()
            .into_iter()
            .map(|i| (i, InteractionType::Send))
            .chain(
                self.receives()
                    .into_iter()
                    .map(|i| (i, InteractionType::Receive)),
            )
            .collect::<Vec<_>>()
    }

    fn permutation_width(&self) -> usize {
        let num_interactions = self.sends().len() + self.receives().len();
        if num_interactions > 0 {
            num_interactions + 1
        } else {
            0
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

        let alphas: Vec<AB::ExprEF> =
            generate_rlc_elements(&self.sends(), &self.receives(), rand_elems[0].into());
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
}

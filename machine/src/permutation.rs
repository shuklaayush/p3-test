use itertools::Itertools;
use p3_air::{ExtensionBuilder, PairBuilder, PermutationAirBuilder, VirtualPairCol};
use p3_field::{AbstractField, ExtensionField, Field, Powers};
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use p3_uni_stark::{StarkGenericConfig, Val};
use std::borrow::Borrow;

use crate::chip::MachineChip;
use crate::interaction::InteractionType;
use crate::util::batch_multiplicative_inverse_allowing_zero;

/// Generate the permutation trace for a chip with the provided machine.
/// This is called only after `generate_trace` has been called on all chips.
pub fn generate_permutation_trace<SC: StarkGenericConfig, C: MachineChip<SC>>(
    chip: &C,
    main: &RowMajorMatrix<Val<SC>>,
    random_elements: Vec<SC::Challenge>,
) -> Option<RowMajorMatrix<SC::Challenge>> {
    let all_interactions = chip.all_interactions();
    if all_interactions.is_empty() {
        return None;
    }
    let alphas = generate_rlc_elements(chip, random_elements[0]);
    let betas = random_elements[1].powers();

    let preprocessed = chip.preprocessed_trace();

    // Compute the reciprocal columns
    //
    // Row: | q_1 | q_2 | q_3 | ... | q_n | \phi |
    // * q_i = \frac{1}{\alpha^i + \sum_j \beta^j * f_{i,j}}
    // * f_{i,j} is the jth main trace column for the ith interaction
    // * \phi is the running sum
    //
    // Note: We can optimize this by combining several reciprocal columns into one (the
    // number is subject to a target constraint degree).
    let perm_width = all_interactions.len() + 1;
    let mut perm_values = Vec::with_capacity(main.height() * perm_width);

    for (n, main_row) in main.rows().enumerate() {
        let main_row = main_row.collect_vec();

        let mut row = vec![SC::Challenge::zero(); perm_width];
        for (m, (interaction, _)) in all_interactions.iter().enumerate() {
            let alpha_m = alphas[interaction.argument_index];
            let preprocessed_row = preprocessed
                .as_ref()
                .map(|preprocessed| {
                    let row = preprocessed.row_slice(n);
                    let row: &[_] = (*row).borrow();
                    row.to_vec()
                })
                .unwrap_or_default();
            row[m] = reduce_row(
                main_row.as_slice(),
                preprocessed_row.as_slice(),
                &interaction.fields,
                alpha_m,
                betas.clone(),
            );
        }
        perm_values.extend(row);
    }
    // TODO: Switch to batch_multiplicative_inverse (not allowing zero)?
    // Zero should be vanishingly unlikely if properly randomized?
    let perm_values = batch_multiplicative_inverse_allowing_zero(perm_values);
    let mut perm = RowMajorMatrix::new(perm_values, perm_width);

    // Compute the running sum column
    let mut phi = vec![SC::Challenge::zero(); perm.height()];
    for (n, (main_row, perm_row)) in main.rows().zip(perm.rows()).enumerate() {
        let main_row = main_row.collect_vec();
        let perm_row = perm_row.collect_vec();

        if n > 0 {
            phi[n] = phi[n - 1];
        }
        let preprocessed_row = preprocessed
            .as_ref()
            .map(|preprocessed| {
                let row = preprocessed.row_slice(n);
                let row: &[_] = (*row).borrow();
                row.to_vec()
            })
            .unwrap_or_default();
        for (m, (interaction, interaction_type)) in all_interactions.iter().enumerate() {
            let mult = interaction
                .count
                .apply::<Val<SC>, Val<SC>>(preprocessed_row.as_slice(), main_row.as_slice());
            match interaction_type {
                InteractionType::Send => {
                    phi[n] += perm_row[m] * mult;
                }
                InteractionType::Receive => {
                    phi[n] -= perm_row[m] * mult;
                }
            }
        }
    }

    for (n, row) in perm.as_view_mut().rows_mut().enumerate() {
        *row.last_mut().unwrap() = phi[n];
    }

    Some(perm)
}

pub fn eval_permutation_constraints<C, SC, AB>(chip: &C, builder: &mut AB, cumulative_sum: AB::EF)
where
    C: MachineChip<SC>,
    SC: StarkGenericConfig,
    AB: PairBuilder<F = Val<SC>> + PermutationAirBuilder<F = Val<SC>, EF = SC::Challenge>,
{
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

    let all_interactions = chip.all_interactions();

    let alphas = generate_rlc_elements(chip, rand_elems[0].into());
    let betas = rand_elems[1].into().powers();

    let lhs = phi_next.into() - phi_local.into();
    let mut rhs = AB::ExprEF::zero();
    let mut phi_0 = AB::ExprEF::zero();
    for (m, (interaction, interaction_type)) in all_interactions.iter().enumerate() {
        // Reciprocal constraints
        let mut rlc = AB::ExprEF::zero();
        for (field, beta) in interaction.fields.iter().zip(betas.clone()) {
            let elem = field.apply::<AB::Expr, AB::Var>(preprocessed_local, main_local);
            rlc += beta * elem;
        }
        rlc += alphas[interaction.argument_index].clone();
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

    // Running sum constraints
    builder.when_transition().assert_eq_ext(lhs, rhs);
    builder
        .when_first_row()
        .assert_eq_ext(*perm_local.last().unwrap(), phi_0);
    builder.when_last_row().assert_eq_ext(
        *perm_local.last().unwrap(),
        AB::ExprEF::from_f(cumulative_sum),
    );
}

fn generate_rlc_elements<SC: StarkGenericConfig, C: MachineChip<SC>, AF: AbstractField>(
    chip: &C,
    random_element: AF,
) -> Vec<AF> {
    random_element
        .powers()
        .skip(1)
        .take(
            chip.sends()
                .into_iter()
                .chain(chip.receives())
                .map(|interaction| interaction.argument_index)
                .max()
                .unwrap_or(0)
                + 1,
        )
        .collect_vec()
}

// TODO: Use Var and Expr type bounds in place of concrete fields so that
// this function can be used in `eval_permutation_constraints`.
fn reduce_row<F, EF>(
    main_row: &[F],
    preprocessed_row: &[F],
    fields: &[VirtualPairCol<F>],
    alpha: EF,
    betas: Powers<EF>,
) -> EF
where
    F: Field,
    EF: ExtensionField<F>,
{
    let mut rlc = EF::zero();
    for (columns, beta) in fields.iter().zip(betas) {
        rlc += beta * columns.apply::<F, F>(preprocessed_row, main_row)
    }
    rlc += alpha;
    rlc
}

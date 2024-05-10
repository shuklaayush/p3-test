use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;

use p3_field::{ExtensionField, Field};
use p3_matrix::{dense::RowMajorMatrix, Matrix};

use super::interaction::{Interaction, InteractionType};
use crate::{batch_multiplicative_inverse_allowing_zero, generate_rlc_elements, reduce_row};

pub fn generate_permutation_trace<F: Field, EF: ExtensionField<F>>(
    preprocessed: &Option<RowMajorMatrix<F>>,
    main: &RowMajorMatrix<F>,
    sends: &[Interaction<F>],
    receives: &[Interaction<F>],
    random_elements: Vec<EF>,
) -> Option<RowMajorMatrix<EF>> {
    let interactions: Vec<_> = sends
        .iter()
        .map(|i| (i, InteractionType::Send))
        .chain(receives.iter().map(|i| (i, InteractionType::Receive)))
        .collect();
    if interactions.is_empty() {
        return None;
    }

    let alphas = generate_rlc_elements(sends, receives, random_elements[0]);
    let betas = random_elements[1].powers();

    // Compute the reciprocal columns
    //
    // Row: | q_1 | q_2 | q_3 | ... | q_n | \phi |
    // * q_i = \frac{1}{\alpha^i + \sum_j \beta^j * f_{i,j}}
    // * f_{i,j} is the jth main trace column for the ith interaction
    // * \phi is the running sum
    //
    // Note: We can optimize this by combining several reciprocal columns into one (the
    // number is subject to a target constraint degree).
    let perm_width = interactions.len() + 1;
    let mut perm_values = Vec::with_capacity(main.height() * perm_width);

    for (n, main_row) in main.rows().enumerate() {
        let main_row: Vec<_> = main_row.collect();

        let mut row = vec![EF::zero(); perm_width];
        for (m, (interaction, _)) in interactions.iter().enumerate() {
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
    let mut phi = vec![EF::zero(); perm.height()];
    for (n, (main_row, perm_row)) in main.rows().zip(perm.rows()).enumerate() {
        let main_row: Vec<_> = main_row.collect();
        let perm_row: Vec<_> = perm_row.collect();

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
        for (m, (interaction, interaction_type)) in interactions.iter().enumerate() {
            let mult = interaction
                .count
                .apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
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

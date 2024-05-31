use alloc::vec::Vec;

use p3_air::VirtualPairCol;
use p3_field::{AbstractExtensionField, AbstractField, Field, Powers};

use crate::interaction::{Interaction, InteractionType};

pub fn generate_rlc_elements<F, EF>(
    interactions: &[(Interaction<F>, InteractionType)],
    random_element: EF,
) -> Vec<EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    random_element
        .powers()
        .skip(1)
        .take(
            interactions
                .iter()
                .map(|(interaction, _)| interaction.argument_index)
                .max()
                .unwrap_or(0)
                + 1,
        )
        .collect()
}

pub fn reduce_row<Var, Expr, ExprEF>(
    preprocessed_row: &[Var],
    main_row: &[Var],
    fields: &[VirtualPairCol<Expr>],
    alpha: ExprEF,
    betas: Powers<ExprEF>,
) -> ExprEF
where
    Var: Into<Expr> + Copy,
    Expr: AbstractField,
    ExprEF: AbstractExtensionField<Expr>,
{
    let mut rlc = ExprEF::zero();
    for (columns, beta) in fields.iter().zip(betas) {
        rlc += beta * columns.apply::<Expr, Var>(preprocessed_row, main_row)
    }
    rlc += alpha;
    rlc
}

/// Calculates and returns the multiplicative inverses of each field element, with zero
/// values remaining unchanged.
pub fn batch_multiplicative_inverse_allowing_zero<F: Field>(values: Vec<F>) -> Vec<F> {
    // Check if values are zero, and construct a new vector with only nonzero values
    let mut nonzero_values = Vec::with_capacity(values.len());
    let mut indices = Vec::with_capacity(values.len());
    for (i, value) in values.iter().cloned().enumerate() {
        if value.is_zero() {
            continue;
        }
        nonzero_values.push(value);
        indices.push(i);
    }

    // Compute the multiplicative inverse of nonzero values
    let inverse_nonzero_values = p3_field::batch_multiplicative_inverse(&nonzero_values);

    // Reconstruct the original vector
    let mut result = values.clone();
    for (i, index) in indices.into_iter().enumerate() {
        result[index] = inverse_nonzero_values[i];
    }

    result
}

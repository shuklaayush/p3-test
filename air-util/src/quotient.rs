use alloc::vec::Vec;

use p3_field::Field;
use p3_interaction::Rap;
use p3_uni_stark::SymbolicExpression;
use p3_util::log2_ceil_usize;
use tracing::instrument;

use crate::folders::SymbolicAirBuilder;

#[instrument(name = "infer log of constraint degree", skip_all)]
pub fn get_quotient_degree<F, A>(air: &A, num_public_values: usize) -> usize
where
    F: Field,
    A: Rap<SymbolicAirBuilder<F>>,
{
    // We pad to at least degree 2, since a quotient argument doesn't make sense with smaller degrees.
    let constraint_degree = get_max_constraint_degree(air, num_public_values).max(2);

    // The quotient's actual degree is approximately (max_constraint_degree - 1) n,
    // where subtracting 1 comes from division by the zerofier.
    // But we pad it to a power of two so that we can efficiently decompose the quotient.
    let d = log2_ceil_usize(constraint_degree - 1);
    1 << d
}

#[instrument(name = "infer constraint degree", skip_all, level = "debug")]
fn get_max_constraint_degree<F, A>(air: &A, num_public_values: usize) -> usize
where
    F: Field,
    A: Rap<SymbolicAirBuilder<F>>,
{
    get_symbolic_constraints(air, num_public_values)
        .iter()
        .map(|c| c.degree_multiple())
        .max()
        .unwrap_or(0)
}

#[instrument(name = "evaluate constraints symbolically", skip_all, level = "debug")]
fn get_symbolic_constraints<F, A>(air: &A, num_public_values: usize) -> Vec<SymbolicExpression<F>>
where
    F: Field,
    A: Rap<SymbolicAirBuilder<F>>,
{
    let mut builder = SymbolicAirBuilder::new(
        air.preprocessed_width(),
        air.width(),
        air.permutation_width().unwrap_or_default(),
        num_public_values,
    );
    air.eval_all(&mut builder);
    builder.constraints()
}

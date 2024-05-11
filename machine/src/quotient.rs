use std::cmp::min;

use itertools::Itertools;
use p3_commit::PolynomialSpace;
use p3_field::{AbstractExtensionField, AbstractField, PackedValue};
use p3_interaction::InteractionAir;
use p3_matrix::{dense::RowMajorMatrixView, stack::VerticalPair, Matrix};
use p3_maybe_rayon::prelude::*;
use p3_stark::prover::ProverConstraintFolder;
use p3_uni_stark::{Domain, PackedChallenge, PackedVal, StarkGenericConfig, Val};
use p3_util::log2_strict_usize;

pub fn quotient_values<SC, A, Mat>(
    air: &A,
    main_domain: Domain<SC>,
    quotient_domain: Domain<SC>,
    preprocessed_trace_on_quotient_domain: Mat,
    main_trace_on_quotient_domain: Mat,
    perm_trace_on_quotient_domain: Mat,
    perm_challenges: &[PackedChallenge<SC>],
    alpha: SC::Challenge,
    cumulative_sum: SC::Challenge,
) -> Vec<SC::Challenge>
where
    SC: StarkGenericConfig,
    A: for<'a> InteractionAir<ProverConstraintFolder<'a, SC>>,
    Mat: Matrix<Val<SC>> + Sync,
{
    let quotient_size = quotient_domain.size();
    let perm_width = perm_trace_on_quotient_domain.width();
    let mut sels = main_domain.selectors_on_coset(quotient_domain);

    let qdb = log2_strict_usize(quotient_domain.size()) - log2_strict_usize(main_domain.size());
    let next_step = 1 << qdb;

    // assert!(quotient_size >= PackedVal::<SC>::WIDTH);
    // We take PackedVal::<SC>::WIDTH worth of values at a time from a quotient_size slice, so we need to
    // pad with default values in the case where quotient_size is smaller than PackedVal::<SC>::WIDTH.
    for _ in quotient_size..PackedVal::<SC>::WIDTH {
        sels.is_first_row.push(Val::<SC>::default());
        sels.is_last_row.push(Val::<SC>::default());
        sels.is_transition.push(Val::<SC>::default());
        sels.inv_zeroifier.push(Val::<SC>::default());
    }

    (0..quotient_size)
        .into_par_iter()
        .step_by(PackedVal::<SC>::WIDTH)
        .flat_map_iter(|i_start| {
            let wrap = |i| i % quotient_size;
            let i_range = i_start..i_start + PackedVal::<SC>::WIDTH;

            let is_first_row = *PackedVal::<SC>::from_slice(&sels.is_first_row[i_range.clone()]);
            let is_last_row = *PackedVal::<SC>::from_slice(&sels.is_last_row[i_range.clone()]);
            let is_transition = *PackedVal::<SC>::from_slice(&sels.is_transition[i_range.clone()]);
            let inv_zeroifier = *PackedVal::<SC>::from_slice(&sels.inv_zeroifier[i_range.clone()]);

            // TODO: Any way to do it without collect?
            let preprocessed_local = preprocessed_trace_on_quotient_domain
                .vertically_packed_row(i_start)
                .collect_vec();
            let preprocessed_next = preprocessed_trace_on_quotient_domain
                .vertically_packed_row(i_start + next_step)
                .collect_vec();

            let main_local = main_trace_on_quotient_domain
                .vertically_packed_row(i_start)
                .collect_vec();
            let main_next = main_trace_on_quotient_domain
                .vertically_packed_row(i_start + next_step)
                .collect_vec();

            // TODO: Use vertically_packed
            let perm_local = (0..perm_width)
                .step_by(SC::Challenge::D)
                .map(|col| {
                    PackedChallenge::<SC>::from_base_fn(|i| {
                        PackedVal::<SC>::from_fn(|offset| {
                            perm_trace_on_quotient_domain.get(wrap(i_start + offset), col + i)
                        })
                    })
                })
                .collect_vec();
            let perm_next = (0..perm_width)
                .step_by(SC::Challenge::D)
                .map(|col| {
                    PackedChallenge::<SC>::from_base_fn(|i| {
                        PackedVal::<SC>::from_fn(|offset| {
                            perm_trace_on_quotient_domain
                                .get(wrap(i_start + next_step + offset), col + i)
                        })
                    })
                })
                .collect_vec();

            let accumulator = PackedChallenge::<SC>::zero();
            let mut folder = ProverConstraintFolder {
                preprocessed: VerticalPair::new(
                    RowMajorMatrixView::new_row(&preprocessed_local),
                    RowMajorMatrixView::new_row(&preprocessed_next),
                ),
                main: VerticalPair::new(
                    RowMajorMatrixView::new_row(&main_local),
                    RowMajorMatrixView::new_row(&main_next),
                ),
                perm: VerticalPair::new(
                    RowMajorMatrixView::new_row(&perm_local),
                    RowMajorMatrixView::new_row(&perm_next),
                ),
                perm_challenges,
                public_values: &vec![],
                // TODO: Check this
                cumulative_sum: PackedChallenge::<SC>::from_base_fn(|i| {
                    PackedVal::<SC>::from(cumulative_sum.as_base_slice()[i])
                }),
                is_first_row,
                is_last_row,
                is_transition,
                alpha,
                accumulator,
            };
            air.eval_all(&mut folder);

            // quotient(x) = constraints(x) / Z_H(x)
            let quotient = folder.accumulator * inv_zeroifier;

            // "Transpose" D packed base coefficients into WIDTH scalar extension coefficients.
            let width = min(PackedVal::<SC>::WIDTH, quotient_size);
            (0..width).map(move |idx_in_packing| {
                let quotient_value = (0..<SC::Challenge as AbstractExtensionField<Val<SC>>>::D)
                    .map(|coeff_idx| quotient.as_base_slice()[coeff_idx].as_slice()[idx_in_packing])
                    .collect_vec();
                SC::Challenge::from_base_slice(&quotient_value)
            })
        })
        .collect()
}

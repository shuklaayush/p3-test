use std::cmp::min;

use itertools::Itertools;
use p3_air::TwoRowMatrixView;
use p3_commit::PolynomialSpace;
use p3_field::{AbstractExtensionField, AbstractField, PackedValue};
use p3_matrix::MatrixGet;
use p3_maybe_rayon::prelude::*;
use p3_uni_stark::{Domain, PackedChallenge, PackedVal, StarkGenericConfig, Val};
use p3_util::log2_strict_usize;

use crate::{
    chip::MachineChip, folder::ProverConstraintFolder, permutation::eval_permutation_constraints,
};

pub fn quotient_values<SC, C, Mat>(
    chip: &C,
    cumulative_sum: SC::Challenge,
    trace_domain: Domain<SC>,
    quotient_domain: Domain<SC>,
    // preprocessed_trace_on_quotient_domain: Mat,
    main_trace_on_quotient_domain: Mat,
    perm_trace_on_quotient_domain: Mat,
    perm_challenges: &[PackedChallenge<SC>],
    alpha: SC::Challenge,
) -> Vec<SC::Challenge>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
    Mat: MatrixGet<Val<SC>> + Sync,
{
    let quotient_size = quotient_domain.size();
    // let preprocessed_width = preprocessed_trace_on_quotient_domain.width();
    let main_width = main_trace_on_quotient_domain.width();
    let perm_width = perm_trace_on_quotient_domain.width();
    let mut sels = trace_domain.selectors_on_coset(quotient_domain);

    let qdb = log2_strict_usize(quotient_domain.size()) - log2_strict_usize(trace_domain.size());
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

            // let preprocessed_local = (0..preprocessed_width)
            //     .map(|col| {
            //         PackedVal::<SC>::from_fn(|offset| {
            //             preprocessed_trace_on_quotient_domain.get(wrap(i_start + offset), col)
            //         })
            //     })
            //     .collect_vec();
            // let preprocessed_next = (0..preprocessed_width)
            //     .map(|col| {
            //         PackedVal::<SC>::from_fn(|offset| {
            //             preprocessed_trace_on_quotient_domain
            //                 .get(wrap(i_start + next_step + offset), col)
            //         })
            //     })
            //     .collect_vec();

            let local = (0..main_width)
                .map(|col| {
                    PackedVal::<SC>::from_fn(|offset| {
                        main_trace_on_quotient_domain.get(wrap(i_start + offset), col)
                    })
                })
                .collect_vec();

            let next = (0..main_width)
                .map(|col| {
                    PackedVal::<SC>::from_fn(|offset| {
                        main_trace_on_quotient_domain.get(wrap(i_start + next_step + offset), col)
                    })
                })
                .collect_vec();

            let perm_local: Vec<_> = (0..perm_width)
                .step_by(SC::Challenge::D)
                .map(|col| {
                    PackedChallenge::<SC>::from_base_fn(|i| {
                        PackedVal::<SC>::from_fn(|offset| {
                            perm_trace_on_quotient_domain.get(wrap(i_start + offset), col + i)
                        })
                    })
                })
                .collect();

            let perm_next: Vec<_> = (0..perm_width)
                .step_by(SC::Challenge::D)
                .map(|col| {
                    PackedChallenge::<SC>::from_base_fn(|i| {
                        PackedVal::<SC>::from_fn(|offset| {
                            perm_trace_on_quotient_domain
                                .get(wrap(i_start + next_step + offset), col + i)
                        })
                    })
                })
                .collect();

            let accumulator = PackedChallenge::<SC>::zero();
            let mut folder = ProverConstraintFolder {
                preprocessed: TwoRowMatrixView {
                    local: &[],
                    next: &[],
                },
                main: TwoRowMatrixView {
                    local: &local,
                    next: &next,
                },
                perm: TwoRowMatrixView {
                    local: &perm_local,
                    next: &perm_next,
                },
                perm_challenges,
                public_values: &vec![],
                cumulative_sum,
                is_first_row,
                is_last_row,
                is_transition,
                alpha,
                accumulator,
            };
            chip.eval(&mut folder);
            eval_permutation_constraints::<_, SC, _>(chip, &mut folder, cumulative_sum);

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

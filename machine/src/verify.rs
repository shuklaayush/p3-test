use itertools::Itertools;
use p3_commit::PolynomialSpace;
use p3_field::{AbstractExtensionField, AbstractField, Field};
use p3_matrix::dense::RowMajorMatrixView;
use p3_matrix::stack::VerticalPair;
use p3_uni_stark::Domain;
use p3_uni_stark::StarkGenericConfig;

use crate::chip::MachineChip;
use crate::error::VerificationError;
use crate::folder::VerifierConstraintFolder;
use crate::permutation::eval_permutation_constraints;
use crate::proof::OpenedValues;

pub fn verify_constraints<SC: StarkGenericConfig, C: MachineChip<SC>>(
    chip: &C,
    opened_values: &OpenedValues<SC::Challenge>,
    cumulative_sum: Option<SC::Challenge>,
    main_domain: Domain<SC>,
    qc_domains: &[Domain<SC>],
    zeta: SC::Challenge,
    alpha: SC::Challenge,
    permutation_challenges: &[SC::Challenge],
) -> Result<(), VerificationError> {
    let zps = qc_domains
        .iter()
        .enumerate()
        .map(|(i, domain)| {
            qc_domains
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, other_domain)| {
                    other_domain.zp_at_point(zeta)
                        * other_domain.zp_at_point(domain.first_point()).inverse()
                })
                .product::<SC::Challenge>()
        })
        .collect_vec();

    let quotient = opened_values
        .quotient_chunks
        .iter()
        .enumerate()
        .map(|(ch_i, ch)| {
            ch.iter()
                .enumerate()
                .map(|(e_i, &c)| zps[ch_i] * SC::Challenge::monomial(e_i) * c)
                .sum::<SC::Challenge>()
        })
        .sum::<SC::Challenge>();

    let sels = main_domain.selectors_at_point(zeta);

    let unflatten = |v: &[SC::Challenge]| {
        v.chunks_exact(SC::Challenge::D)
            .map(|chunk| {
                chunk
                    .iter()
                    .enumerate()
                    .map(|(e_i, &c)| SC::Challenge::monomial(e_i) * c)
                    .sum()
            })
            .collect::<Vec<SC::Challenge>>()
    };

    let (preprocessed_local, preprocessed_next) =
        if let Some(opened_values) = &opened_values.preprocessed {
            (opened_values.local.clone(), opened_values.next.clone())
        } else {
            (vec![], vec![])
        };

    let (perm_local, perm_next) = if let Some(opened_values) = &opened_values.permutation {
        (
            unflatten(&opened_values.local),
            unflatten(&opened_values.next),
        )
    } else {
        (vec![], vec![])
    };

    let mut folder: VerifierConstraintFolder<'_, SC> = VerifierConstraintFolder {
        preprocessed: VerticalPair::new(
            RowMajorMatrixView::new_row(&preprocessed_local),
            RowMajorMatrixView::new_row(&preprocessed_next),
        ),
        main: VerticalPair::new(
            RowMajorMatrixView::new_row(&opened_values.main.local),
            RowMajorMatrixView::new_row(&opened_values.main.next),
        ),
        perm: VerticalPair::new(
            RowMajorMatrixView::new_row(&perm_local),
            RowMajorMatrixView::new_row(&perm_next),
        ),
        perm_challenges: permutation_challenges,
        public_values: &vec![],
        is_first_row: sels.is_first_row,
        is_last_row: sels.is_last_row,
        is_transition: sels.is_transition,
        alpha,
        accumulator: SC::Challenge::zero(),
    };
    chip.eval(&mut folder);
    if let Some(cumulative_sum) = cumulative_sum {
        eval_permutation_constraints::<_, SC, _>(chip, &mut folder, cumulative_sum);
    }

    let folded_constraints = folder.accumulator;
    // Finally, check that
    //     folded_constraints(zeta) / Z_H(zeta) = quotient(zeta)
    if folded_constraints * sels.inv_zeroifier != quotient {
        return Err(VerificationError::OodEvaluationMismatch);
    }

    Ok(())
}

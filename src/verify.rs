use p3_air::TwoRowMatrixView;
use p3_commit::PolynomialSpace;
use p3_field::{AbstractExtensionField, AbstractField, Field};
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
    cumulative_sum: SC::Challenge,
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
        .collect::<Vec<_>>();

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

    let mut folder: VerifierConstraintFolder<'_, SC> = VerifierConstraintFolder {
        preprocessed: TwoRowMatrixView {
            local: &opened_values.preprocessed_local,
            next: &opened_values.preprocessed_next,
        },
        main: TwoRowMatrixView {
            local: &opened_values.trace_local,
            next: &opened_values.trace_next,
        },
        perm: TwoRowMatrixView {
            local: &unflatten(&opened_values.permutation_local),
            next: &unflatten(&opened_values.permutation_next),
        },
        perm_challenges: &permutation_challenges,
        public_values: &vec![],
        is_first_row: sels.is_first_row,
        is_last_row: sels.is_last_row,
        is_transition: sels.is_transition,
        alpha,
        accumulator: SC::Challenge::zero(),
    };
    chip.eval(&mut folder);
    eval_permutation_constraints::<_, SC, _>(chip, &mut folder, cumulative_sum);

    let folded_constraints = folder.accumulator;
    // Finally, check that
    //     folded_constraints(zeta) / Z_H(zeta) = quotient(zeta)
    if folded_constraints * sels.inv_zeroifier != quotient {
        return Err(VerificationError::OodEvaluationMismatch);
    }

    Ok(())
}

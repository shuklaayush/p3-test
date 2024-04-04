use p3_air::TwoRowMatrixView;
use p3_field::{AbstractField, Field};
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::{Matrix, MatrixRowSlices};
use p3_maybe_rayon::prelude::IntoParallelIterator;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::chip::MachineChip;
use crate::debug_builder::DebugConstraintBuilder;
use crate::permutation::eval_permutation_constraints;

/// Check that all constraints vanish on the subgroup.
pub fn check_constraints<C, SC>(
    chip: &C,
    main: &RowMajorMatrix<Val<SC>>,
    perm: &RowMajorMatrix<SC::Challenge>,
    perm_challenges: &[SC::Challenge],
    public_values: &Vec<Val<SC>>,
) where
    C: MachineChip<SC>,
    SC: StarkGenericConfig,
{
    assert_eq!(main.height(), perm.height());
    let height = main.height();
    if height == 0 {
        return;
    }

    let preprocessed = chip.preprocessed_trace();

    let cumulative_sum = *perm.row_slice(perm.height() - 1).last().unwrap();

    // Check that constraints are satisfied.
    (0..height).into_par_iter().for_each(|i| {
        let i_next = (i + 1) % height;

        let main_local = main.row_slice(i);
        let main_next = main.row_slice(i_next);
        let preprocessed_local = if preprocessed.is_some() {
            preprocessed.as_ref().unwrap().row_slice(i)
        } else {
            &[]
        };
        let preprocessed_next = if preprocessed.is_some() {
            preprocessed.as_ref().unwrap().row_slice(i_next)
        } else {
            &[]
        };
        let perm_local = perm.row_slice(i);
        let perm_next = perm.row_slice(i_next);

        let mut builder = DebugConstraintBuilder {
            row_index: i,
            main: TwoRowMatrixView {
                local: main_local,
                next: main_next,
            },
            preprocessed: TwoRowMatrixView {
                local: preprocessed_local,
                next: preprocessed_next,
            },
            perm: TwoRowMatrixView {
                local: perm_local,
                next: perm_next,
            },
            perm_challenges,
            public_values,
            is_first_row: Val::<SC>::zero(),
            is_last_row: Val::<SC>::zero(),
            is_transition: Val::<SC>::one(),
        };
        if i == 0 {
            builder.is_first_row = Val::<SC>::one();
        }
        if i == height - 1 {
            builder.is_last_row = Val::<SC>::one();
            builder.is_transition = Val::<SC>::zero();
        }

        chip.eval(&mut builder);
        eval_permutation_constraints(chip, &mut builder, cumulative_sum);
    });
}

/// Check that the combined cumulative sum across all lookup tables is zero.
pub fn check_cumulative_sums<Challenge: Field>(perms: &[RowMajorMatrix<Challenge>]) {
    let sum: Challenge = perms
        .iter()
        .map(|perm| *perm.row_slice(perm.height() - 1).last().unwrap())
        .sum();
    assert_eq!(sum, Challenge::zero());
}

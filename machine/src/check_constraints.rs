use std::borrow::Borrow;

use p3_field::{AbstractField, Field};
use p3_matrix::dense::{RowMajorMatrix, RowMajorMatrixView};
use p3_matrix::stack::VerticalPair;
use p3_matrix::Matrix;
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

        let (main_local, main_next) = (main.row_slice(i), main.row_slice(i_next));
        let (preprocessed_local, preprocessed_next) = preprocessed
            .as_ref()
            .map(|preprocessed| {
                let local = preprocessed.row_slice(i);
                let next = preprocessed.row_slice(i_next);
                let local: &[_] = (*local).borrow();
                let next: &[_] = (*next).borrow();
                (local.to_vec(), next.to_vec())
            })
            .unwrap_or_default();
        let (perm_local, perm_next) = (perm.row_slice(i), perm.row_slice(i_next));

        let mut builder = DebugConstraintBuilder {
            row_index: i,
            main: VerticalPair::new(
                RowMajorMatrixView::new_row(&*main_local),
                RowMajorMatrixView::new_row(&*main_next),
            ),
            preprocessed: VerticalPair::new(
                RowMajorMatrixView::new_row(&*preprocessed_local),
                RowMajorMatrixView::new_row(&*preprocessed_next),
            ),
            perm: VerticalPair::new(
                RowMajorMatrixView::new_row(&*perm_local),
                RowMajorMatrixView::new_row(&*perm_next),
            ),
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

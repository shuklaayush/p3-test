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
    preprocessed: &Option<RowMajorMatrix<Val<SC>>>,
    main: &RowMajorMatrix<Val<SC>>,
    perm: &Option<RowMajorMatrix<SC::Challenge>>,
    perm_challenges: &[SC::Challenge],
    cumulative_sum: Option<SC::Challenge>,
    public_values: &[Val<SC>],
) where
    C: MachineChip<SC>,
    SC: StarkGenericConfig,
{
    let height = main.height();
    if let Some(perm) = perm {
        assert_eq!(perm.height(), height);
    }

    // Check that constraints are satisfied.
    (0..height).into_par_iter().for_each(|i| {
        let i_next = (i + 1) % height;

        let (preprocessed_local, preprocessed_next) = preprocessed
            .as_ref()
            .map(|preprocessed| {
                (
                    preprocessed.row_slice(i).to_vec(),
                    preprocessed.row_slice(i_next).to_vec(),
                )
            })
            .unwrap_or((vec![], vec![]));
        let (main_local, main_next) = (main.row_slice(i), main.row_slice(i_next));
        let (perm_local, perm_next) = perm
            .as_ref()
            .map(|perm| (perm.row_slice(i).to_vec(), perm.row_slice(i_next).to_vec()))
            .unwrap_or((vec![], vec![]));

        let mut builder = DebugConstraintBuilder {
            row_index: i,
            preprocessed: VerticalPair::new(
                RowMajorMatrixView::new_row(preprocessed_local.as_slice()),
                RowMajorMatrixView::new_row(preprocessed_next.as_slice()),
            ),
            main: VerticalPair::new(
                RowMajorMatrixView::new_row(&*main_local),
                RowMajorMatrixView::new_row(&*main_next),
            ),
            perm: VerticalPair::new(
                RowMajorMatrixView::new_row(perm_local.as_slice()),
                RowMajorMatrixView::new_row(perm_next.as_slice()),
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
        if let Some(cumulative_sum) = cumulative_sum {
            eval_permutation_constraints(chip, &mut builder, cumulative_sum);
        }
    });
}

/// Check that the combined cumulative sum across all lookup tables is zero.
pub fn check_cumulative_sums<Challenge: Field>(perms: &[Option<RowMajorMatrix<Challenge>>]) {
    let sum: Challenge = perms
        .iter()
        .flatten()
        .map(|perm| *perm.row_slice(perm.height() - 1).last().unwrap())
        .sum();
    assert_eq!(sum, Challenge::zero());
}

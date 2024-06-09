use alloc::vec;
use alloc::vec::Vec;
use p3_air::Air;

use p3_field::Field;
use p3_matrix::dense::RowMajorMatrixView;
use p3_matrix::stack::VerticalPair;
use p3_matrix::Matrix;
use p3_maybe_rayon::prelude::IntoParallelIterator;

use crate::{
    folders::{air::TrackingConstraintBuilder, EntriesLog},
    util::{TraceEntry, TrackedFieldVariable},
};

pub fn track_constraints<F, A>(
    air: &A,
    preprocessed: &Option<RowMajorMatrixView<F>>,
    main: &Option<RowMajorMatrixView<F>>,
    public_values: &[F],
) -> EntriesLog<TraceEntry>
where
    F: Field,
    A: for<'a> Air<TrackingConstraintBuilder<'a, F>>,
{
    let height = match (main.as_ref(), preprocessed.as_ref()) {
        (Some(main), Some(preprocessed)) => core::cmp::max(main.height(), preprocessed.height()),
        (Some(main), None) => main.height(),
        (None, Some(preprocessed)) => preprocessed.height(),
        (None, None) => 0,
    };
    let mut entries = EntriesLog::<TraceEntry>::default();
    (0..height).into_par_iter().for_each(|i| {
        let i_next = (i + 1) % height;

        let (preprocessed_local, preprocessed_next) = preprocessed
            .as_ref()
            .map(|preprocessed| {
                (
                    preprocessed
                        .row_slice(i)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            let entry = TraceEntry::Preprocessed { row: i, col: j };
                            TrackedFieldVariable::new(*x, entry)
                        })
                        .collect::<Vec<_>>(),
                    preprocessed
                        .row_slice(i_next)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            let entry = TraceEntry::Preprocessed {
                                row: i_next,
                                col: j,
                            };
                            TrackedFieldVariable::new(*x, entry)
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or((vec![], vec![]));
        let (main_local, main_next) = main
            .as_ref()
            .map(|main| {
                (
                    main.row_slice(i)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            let entry = TraceEntry::Main { row: i, col: j };
                            TrackedFieldVariable::new(*x, entry)
                        })
                        .collect::<Vec<_>>(),
                    main.row_slice(i_next)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            let entry = TraceEntry::Main {
                                row: i_next,
                                col: j,
                            };
                            TrackedFieldVariable::new(*x, entry)
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or((vec![], vec![]));

        let public_values = public_values
            .iter()
            .enumerate()
            .map(|(j, x)| TrackedFieldVariable::new(*x, TraceEntry::Public { index: j }))
            .collect::<Vec<_>>();

        let mut builder = TrackingConstraintBuilder {
            entries: EntriesLog::default(),
            preprocessed: VerticalPair::new(
                RowMajorMatrixView::new_row(preprocessed_local.as_slice()),
                RowMajorMatrixView::new_row(preprocessed_next.as_slice()),
            ),
            main: VerticalPair::new(
                RowMajorMatrixView::new_row(&*main_local),
                RowMajorMatrixView::new_row(&*main_next),
            ),
            public_values: public_values.as_slice(),
            is_first_row: F::zero(),
            is_last_row: F::zero(),
            is_transition: F::one(),
        };
        if i == 0 {
            builder.is_first_row = F::one();
        }
        if i == height - 1 {
            builder.is_last_row = F::one();
            builder.is_transition = F::zero();
        }

        air.eval(&mut builder);
        entries.extend(&builder.entries);
    });

    entries
}

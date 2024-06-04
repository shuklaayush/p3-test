use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;

use hashbrown::HashMap;
use p3_field::{ExtensionField, Field};
use p3_interaction::{Bus, InteractionType, Rap};
use p3_matrix::dense::RowMajorMatrixView;
use p3_matrix::stack::VerticalPair;
use p3_matrix::Matrix;
use p3_maybe_rayon::prelude::IntoParallelIterator;

use crate::util::{MultiTraceEntry, TrackedFieldExpression};
use crate::{
    folders::{DebugConstraintBuilder, TrackingConstraintBuilder},
    util::{TraceEntry, TrackedFieldVariable},
};

/// Check that all constraints vanish on the subgroup.
pub fn check_constraints<F, EF, A>(
    air: &A,
    preprocessed: &Option<RowMajorMatrixView<F>>,
    main: &Option<RowMajorMatrixView<F>>,
    perm: &Option<RowMajorMatrixView<EF>>,
    perm_challenges: [EF; 2],
    cumulative_sum: Option<EF>,
    public_values: &[F],
) where
    F: Field,
    EF: ExtensionField<F>,
    A: for<'a> Rap<DebugConstraintBuilder<'a, F, EF>>,
{
    let height = match (main.as_ref(), preprocessed.as_ref()) {
        (Some(main), Some(preprocessed)) => core::cmp::max(main.height(), preprocessed.height()),
        (Some(main), None) => main.height(),
        (None, Some(preprocessed)) => preprocessed.height(),
        (None, None) => 0,
    };

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
        let (main_local, main_next) = main
            .as_ref()
            .map(|main| (main.row_slice(i).to_vec(), main.row_slice(i_next).to_vec()))
            .unwrap_or((vec![], vec![]));
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
            permutation: VerticalPair::new(
                RowMajorMatrixView::new_row(perm_local.as_slice()),
                RowMajorMatrixView::new_row(perm_next.as_slice()),
            ),
            perm_challenges,
            public_values,
            cumulative_sum: cumulative_sum.unwrap_or_default(),
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

        air.eval_all(&mut builder);
    });
}

pub fn check_cumulative_sums<F, EF, A, B>(
    airs: &[A],
    preprocessed: &[Option<RowMajorMatrixView<F>>],
    main: &[Option<RowMajorMatrixView<F>>],
    permutation: &[Option<RowMajorMatrixView<EF>>],
) where
    F: Field,
    EF: ExtensionField<F>,
    A: for<'a> Rap<DebugConstraintBuilder<'a, F, EF>>,
    B: Bus,
{
    let mut sums = BTreeMap::new();
    for (i, air) in airs.iter().enumerate() {
        for (j, (interaction, interaction_type)) in air.all_interactions().iter().enumerate() {
            if let Some(permutation) = permutation[i].as_ref() {
                for (n, perm_row) in permutation.rows().enumerate() {
                    let preprocessed_row = preprocessed[i]
                        .as_ref()
                        .map(|preprocessed| {
                            let row = preprocessed.row_slice(n);
                            let row: &[_] = (*row).borrow();
                            row.to_vec()
                        })
                        .unwrap_or_default();
                    let main_row = main[i]
                        .as_ref()
                        .map(|main| {
                            let row = main.row_slice(n);
                            let row: &[_] = (*row).borrow();
                            row.to_vec()
                        })
                        .unwrap_or_default();
                    let perm_row: Vec<_> = perm_row.collect();
                    let mult = interaction
                        .count
                        .apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                    let val = match interaction_type {
                        InteractionType::Send => perm_row[j] * mult,
                        InteractionType::Receive => -perm_row[j] * mult,
                    };
                    sums.entry(interaction.argument_index)
                        .and_modify(|c| *c += val)
                        .or_insert(val);
                }
            }
        }
    }
    for (i, sum) in sums {
        assert_eq!(sum, EF::zero(), "{} cumulative sum is not zero", B::from(i));
    }

    // Check cumulative sums
    let sum: EF = permutation
        .iter()
        .flatten()
        .map(|perm| *perm.row_slice(perm.height() - 1).last().unwrap())
        .sum();
    assert_eq!(sum, EF::zero());
}

pub fn track_failing_constraints<F, EF, A>(
    air: &A,
    preprocessed: &Option<RowMajorMatrixView<F>>,
    main: &Option<RowMajorMatrixView<F>>,
    permutation: &Option<RowMajorMatrixView<EF>>,
    perm_challenges: [EF; 2],
    cumulative_sum: Option<EF>,
    public_values: &[F],
) -> BTreeSet<TraceEntry>
where
    F: Field,
    EF: ExtensionField<F>,
    A: for<'a> Rap<TrackingConstraintBuilder<'a, F, EF>>,
{
    let height = match (main.as_ref(), preprocessed.as_ref()) {
        (Some(main), Some(preprocessed)) => core::cmp::max(main.height(), preprocessed.height()),
        (Some(main), None) => main.height(),
        (None, Some(preprocessed)) => preprocessed.height(),
        (None, None) => 0,
    };
    if let Some(perm) = permutation {
        assert_eq!(perm.height(), height);
    }

    let mut entries = BTreeSet::new();
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
                            TrackedFieldVariable::new(
                                *x,
                                TraceEntry::Preprocessed { row: i, col: j },
                            )
                        })
                        .collect::<Vec<_>>(),
                    preprocessed
                        .row_slice(i_next)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            TrackedFieldVariable::new(
                                *x,
                                TraceEntry::Preprocessed { row: i, col: j },
                            )
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
                            TrackedFieldVariable::new(*x, TraceEntry::Main { row: i, col: j })
                        })
                        .collect::<Vec<_>>(),
                    main.row_slice(i_next)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            TrackedFieldVariable::new(*x, TraceEntry::Main { row: i, col: j })
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap_or((vec![], vec![]));
        let (permutation_local, permutation_next) = permutation
            .as_ref()
            .map(|permutation| {
                (
                    permutation
                        .row_slice(i)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            TrackedFieldVariable::new(
                                *x,
                                TraceEntry::Permutation { row: i, col: j },
                            )
                        })
                        .collect::<Vec<_>>(),
                    permutation
                        .row_slice(i_next)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            TrackedFieldVariable::new(
                                *x,
                                TraceEntry::Permutation { row: i, col: j },
                            )
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
        let perm_challenges = perm_challenges.map(|x| TrackedFieldVariable::new_untracked(x));
        let cumulative_sum = cumulative_sum.map(|x| TrackedFieldVariable::new_untracked(x));

        let mut builder = TrackingConstraintBuilder {
            entries: BTreeSet::new(),
            preprocessed: VerticalPair::new(
                RowMajorMatrixView::new_row(preprocessed_local.as_slice()),
                RowMajorMatrixView::new_row(preprocessed_next.as_slice()),
            ),
            main: VerticalPair::new(
                RowMajorMatrixView::new_row(&*main_local),
                RowMajorMatrixView::new_row(&*main_next),
            ),
            permutation: VerticalPair::new(
                RowMajorMatrixView::new_row(&*permutation_local),
                RowMajorMatrixView::new_row(&*permutation_next),
            ),
            public_values: public_values.as_slice(),
            perm_challenges,
            cumulative_sum: cumulative_sum.unwrap_or_default(),
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

        air.eval_all(&mut builder);
        entries.extend(builder.entries);
    });

    entries
}

pub fn track_failing_interactions<F, EF, A>(
    airs: &[A],
    preprocessed: &[Option<RowMajorMatrixView<F>>],
    main: &[Option<RowMajorMatrixView<F>>],
) -> Vec<BTreeSet<TraceEntry>>
where
    F: Field,
    EF: ExtensionField<F>,
    A: for<'a> Rap<DebugConstraintBuilder<'a, F, EF>>,
{
    let mut bus_counts = BTreeMap::new();
    let mut entries = vec![BTreeSet::new(); airs.len()];
    for (i, air) in airs.iter().enumerate() {
        let preprocessed_i = preprocessed[i].as_ref();
        let main_i = main[i].as_ref();
        for (j, (interaction, interaction_type)) in air.all_interactions().iter().enumerate() {
            let preprocessed_height = preprocessed_i.map_or(0, |t| t.height());
            let main_height = main_i.map_or(0, |t| t.height());
            let height = preprocessed_height.max(main_height);

            for n in 0..height {
                let preprocessed_row = preprocessed_i
                    .map(|preprocessed| {
                        let row = preprocessed.row_slice(n);
                        let row: &[_] = (*row).borrow();
                        row.iter()
                            .enumerate()
                            .map(|(k, x)| {
                                TrackedFieldVariable::new(
                                    *x,
                                    MultiTraceEntry::Preprocessed {
                                        trace: i,
                                        row: n,
                                        col: k,
                                    },
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let main_row = main_i
                    .map(|main| {
                        let row = main.row_slice(n);
                        let row: &[_] = (*row).borrow();
                        row.iter()
                            .enumerate()
                            .map(|(k, x)| {
                                TrackedFieldVariable::new(
                                    *x,
                                    MultiTraceEntry::Main {
                                        trace: i,
                                        row: n,
                                        col: k,
                                    },
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let fields = interaction
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(k, f)| {
                        let mut expr = f.apply::<TrackedFieldExpression<F, MultiTraceEntry>, _>(
                            preprocessed_row.as_slice(),
                            main_row.as_slice(),
                        );
                        expr.origin.insert(MultiTraceEntry::VirtualColumnField {
                            trace: i,
                            row: n,
                            interaction: j,
                            field: k,
                        });
                        expr
                    })
                    .collect::<Vec<_>>();
                let mut mult = interaction
                    .count
                    .apply::<TrackedFieldExpression<F, MultiTraceEntry>, _>(
                        preprocessed_row.as_slice(),
                        main_row.as_slice(),
                    );
                mult.origin.insert(MultiTraceEntry::VirtualColumnCount {
                    trace: i,
                    row: n,
                    interaction: j,
                });
                let val = match interaction_type {
                    InteractionType::Send => mult,
                    InteractionType::Receive => -mult,
                };
                let values = fields.iter().map(|f| f.value).collect::<Vec<_>>();
                bus_counts
                    .entry(interaction.argument_index)
                    .or_insert_with(HashMap::new)
                    .entry(values)
                    .and_modify(|c| *c += val.clone())
                    .or_insert(val);
            }
        }
    }

    for counts in bus_counts.into_values() {
        for sum in counts.into_values() {
            if !sum.value.is_zero() {
                for entry in sum.origin.into_iter() {
                    match entry {
                        MultiTraceEntry::Preprocessed { trace, row, col } => {
                            entries[trace].insert(TraceEntry::Preprocessed { row, col });
                        }
                        MultiTraceEntry::Main { trace, row, col } => {
                            entries[trace].insert(TraceEntry::Main { row, col });
                        }
                        MultiTraceEntry::VirtualColumnCount {
                            trace,
                            row,
                            interaction,
                        } => {
                            entries[trace]
                                .insert(TraceEntry::VirtualColumnCount { row, interaction });
                        }
                        MultiTraceEntry::VirtualColumnField {
                            trace,
                            row,
                            interaction,
                            field,
                        } => {
                            entries[trace].insert(TraceEntry::VirtualColumnField {
                                row,
                                interaction,
                                field,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    entries
}

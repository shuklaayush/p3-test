use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;

use hashbrown::HashMap;
use p3_field::{ExtensionField, Field};
use p3_interaction::{InteractionType, Rap, NUM_PERM_CHALLENGES};
use p3_matrix::dense::RowMajorMatrixView;
use p3_matrix::stack::VerticalPair;
use p3_matrix::Matrix;
use p3_maybe_rayon::prelude::IntoParallelIterator;

use crate::folders::EntriesLog;
use crate::util::{MultiTraceEntry, TrackedFieldExpression};
use crate::{
    folders::rap::{DebugConstraintBuilder, TrackingConstraintBuilder},
    util::{TraceEntry, TrackedFieldVariable},
};

pub fn track_constraints<F, EF, A>(
    air: &A,
    preprocessed: &Option<RowMajorMatrixView<F>>,
    main: &Option<RowMajorMatrixView<F>>,
    permutation: &Option<RowMajorMatrixView<EF>>,
    perm_challenges: [EF; NUM_PERM_CHALLENGES],
    cumulative_sum: Option<EF>,
    public_values: &[F],
) -> EntriesLog<TraceEntry>
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
        let (permutation_local, permutation_next) = permutation
            .as_ref()
            .map(|permutation| {
                (
                    permutation
                        .row_slice(i)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            let entry = TraceEntry::Permutation { row: i, col: j };
                            TrackedFieldVariable::new(*x, entry)
                        })
                        .collect::<Vec<_>>(),
                    permutation
                        .row_slice(i_next)
                        .iter()
                        .enumerate()
                        .map(|(j, x)| {
                            let entry = TraceEntry::Permutation {
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
        let perm_challenges = perm_challenges.map(|x| TrackedFieldVariable::new_untracked(x));
        let cumulative_sum = cumulative_sum.map(|x| TrackedFieldVariable::new_untracked(x));

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
        entries.extend(&builder.entries);
    });

    entries
}

pub fn track_interactions<F, EF, A>(
    airs: &[A],
    preprocessed: &[Option<RowMajorMatrixView<F>>],
    main: &[Option<RowMajorMatrixView<F>>],
) -> Vec<EntriesLog<TraceEntry>>
where
    F: Field,
    EF: ExtensionField<F>,
    A: for<'a> Rap<DebugConstraintBuilder<'a, F, EF>>,
{
    let mut bus_counts = BTreeMap::new();
    let mut entries = vec![EntriesLog::default(); airs.len()];
    for (i, air) in airs.iter().enumerate() {
        let preprocessed_i = preprocessed[i].as_ref();
        let main_i = main[i].as_ref();
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
                            let entry = MultiTraceEntry::Preprocessed {
                                trace: i,
                                row: n,
                                col: k,
                            };
                            TrackedFieldVariable::new(*x, entry)
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
                            let entry = MultiTraceEntry::Main {
                                trace: i,
                                row: n,
                                col: k,
                            };
                            TrackedFieldVariable::new(*x, entry)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for (j, (interaction, interaction_type)) in air.all_interactions().iter().enumerate() {
                let entry = MultiTraceEntry::VirtualColumnCount {
                    trace: i,
                    row: n,
                    interaction: j,
                };
                entries[i].constrained.insert(TraceEntry::from(entry));
                let mut mult = interaction
                    .count
                    .apply::<TrackedFieldExpression<F, MultiTraceEntry>, _>(
                        preprocessed_row.as_slice(),
                        main_row.as_slice(),
                    );
                mult = match interaction_type {
                    InteractionType::Send => mult,
                    InteractionType::Receive => -mult,
                };
                mult.value_origin.insert(entry);
                entries[i].constrained.extend(
                    mult.constraint_origin
                        .iter()
                        .map(|entry| TraceEntry::from(*entry)),
                );
                for (k, field) in interaction.fields.iter().enumerate() {
                    // Add virtual column field
                    let entry = MultiTraceEntry::VirtualColumnField {
                        trace: i,
                        row: n,
                        interaction: j,
                        field: k,
                    };
                    entries[i].constrained.insert(TraceEntry::from(entry));
                    // Add origin fields
                    let mut expr = field.apply::<TrackedFieldExpression<F, MultiTraceEntry>, _>(
                        preprocessed_row.as_slice(),
                        main_row.as_slice(),
                    );
                    expr.value_origin.insert(entry);
                    entries[i].constrained.extend(
                        expr.constraint_origin
                            .iter()
                            .map(|entry| TraceEntry::from(*entry)),
                    );

                    let field_val = expr.value;
                    let val = mult.clone() * expr;
                    bus_counts
                        .entry(interaction.argument_index)
                        .or_insert_with(HashMap::new)
                        .entry((k, field_val))
                        .and_modify(|c| *c += val.clone())
                        .or_insert(val);
                }
            }
        }
    }

    for counts in bus_counts.into_values() {
        for sum in counts.into_values() {
            if !sum.value.is_zero() {
                for entry in sum.value_origin.into_iter() {
                    match entry {
                        MultiTraceEntry::Preprocessed { trace, row, col } => {
                            let entry = TraceEntry::Preprocessed { row, col };
                            entries[trace].failing.insert(entry);
                        }
                        MultiTraceEntry::Main { trace, row, col } => {
                            let entry = TraceEntry::Main { row, col };
                            entries[trace].failing.insert(entry);
                        }
                        MultiTraceEntry::VirtualColumnCount {
                            trace,
                            row,
                            interaction,
                        } => {
                            let entry = TraceEntry::VirtualColumnCount { row, interaction };
                            entries[trace].failing.insert(entry);
                        }
                        MultiTraceEntry::VirtualColumnField {
                            trace,
                            row,
                            interaction,
                            field,
                        } => {
                            let entry = TraceEntry::VirtualColumnField {
                                row,
                                interaction,
                                field,
                            };
                            entries[trace].failing.insert(entry);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    entries
}

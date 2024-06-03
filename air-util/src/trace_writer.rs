use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::vec::Vec;

use core::borrow::Borrow;
use core::error::Error;

use p3_field::{ExtensionField, Field, PrimeField32};
use p3_interaction::{Interaction, InteractionType};
use p3_matrix::{dense::RowMajorMatrixView, Matrix};
use rust_xlsxwriter::{Color, Format, Worksheet};

use crate::util::{ColumnEntry, TraceEntry};

pub trait TraceWriter<F: Field, EF: ExtensionField<F>> {
    fn preprocessed_headers(&self) -> Vec<String> {
        // TODO: Assert preprocessed trace is None here
        vec![]
    }

    fn headers(&self) -> Vec<String>;

    fn write_traces_to_worksheet(
        &self,
        ws: &mut Worksheet,
        preprocessed_trace: &Option<RowMajorMatrixView<F>>,
        main_trace: &Option<RowMajorMatrixView<F>>,
        interactions: Vec<(Interaction<F>, InteractionType)>,
        entries: BTreeSet<TraceEntry>,
    ) -> Result<(), Box<dyn Error>>
    where
        F: PrimeField32,
    {
        let column_entries = BTreeSet::from_iter(entries.iter().map(|entry| match entry {
            TraceEntry::None => ColumnEntry::None,
            TraceEntry::Preprocessed { col, .. } => ColumnEntry::Preprocessed { col: *col },
            TraceEntry::Main { col, .. } => ColumnEntry::Main { col: *col },
            TraceEntry::Permutation { col, .. } => ColumnEntry::Permutation { col: *col },
            TraceEntry::VirtualColumnCount { interaction, .. } => ColumnEntry::VirtualColumnCount {
                interaction: *interaction,
            },
            TraceEntry::VirtualColumnField {
                interaction, field, ..
            } => ColumnEntry::VirtualColumnField {
                interaction: *interaction,
                field: *field,
            },
            TraceEntry::Public { index } => ColumnEntry::Public { index: *index },
        }));

        let mut offset = 0;
        let preprocessed_headers = self.preprocessed_headers();
        if !preprocessed_headers.is_empty() {
            for (j, header) in preprocessed_headers.iter().enumerate() {
                let format = {
                    if column_entries.contains(&ColumnEntry::Preprocessed { col: j }) {
                        Format::new().set_background_color(Color::Red)
                    } else {
                        Format::new()
                    }
                };
                ws.write_with_format(0, offset, header, &format)?;
                offset += 1;
            }
            // Blank column
            offset += 1;
        }

        let main_headers = self.headers();
        if !main_headers.is_empty() {
            for (j, header) in self.headers().iter().enumerate() {
                let format = {
                    if column_entries.contains(&ColumnEntry::Main { col: j }) {
                        Format::new().set_background_color(Color::Red)
                    } else {
                        Format::new()
                    }
                };
                ws.write_with_format(0, offset, header, &format)?;
                offset += 1;
            }
            // Blank column
            offset += 1;
        }

        let mut num_receives = 0;
        let mut num_sends = 0;
        for (j, interaction) in interactions.iter().enumerate() {
            let format = {
                if column_entries.contains(&ColumnEntry::VirtualColumnCount { interaction: j }) {
                    Format::new().set_background_color(Color::Red)
                } else {
                    Format::new()
                }
            };
            let (ty, ty_offset) = match interaction.1 {
                InteractionType::Receive => {
                    let out = ("receive", num_receives);
                    num_receives += 1;
                    out
                }
                InteractionType::Send => {
                    let out = ("send", num_sends);
                    num_sends += 1;
                    out
                }
            };

            let prefix = format!("{}[{}]", ty, ty_offset);
            ws.write_with_format(0, offset, format!("{prefix}.count"), &format)?;
            offset += 1;

            for k in 0..interaction.0.fields.len() {
                let format = {
                    if column_entries.contains(&ColumnEntry::VirtualColumnField {
                        interaction: j,
                        field: k,
                    }) {
                        Format::new().set_background_color(Color::Red)
                    } else {
                        Format::new()
                    }
                };
                ws.write_with_format(0, offset, format!("{prefix}[{k}]"), &format)?;
                offset += 1;
            }
            // Blank column
            offset += 1;
        }

        let preprocessed_height = preprocessed_trace.as_ref().map_or(0, |t| t.height());
        let main_height = main_trace.as_ref().map_or(0, |t| t.height());
        let height = preprocessed_height.max(main_height);
        for i in 0..height {
            let mut offset = 0;
            if let Some(preprocessed_trace) = preprocessed_trace {
                for j in 0..preprocessed_trace.width() {
                    let format = {
                        if entries.contains(&TraceEntry::Preprocessed { row: i, col: j }) {
                            Format::new().set_background_color(Color::Red)
                        } else {
                            Format::new()
                        }
                    };
                    ws.write_number_with_format(
                        i as u32 + 1,
                        offset,
                        preprocessed_trace.get(i, j).as_canonical_u32() as f64,
                        &format,
                    )?;
                    offset += 1;
                }
                // Blank column
                offset += 1;
            }

            if let Some(main_trace) = main_trace {
                for j in 0..main_trace.width() {
                    let format = {
                        if entries.contains(&TraceEntry::Main { row: i, col: j }) {
                            Format::new().set_background_color(Color::Red)
                        } else {
                            Format::new()
                        }
                    };
                    ws.write_number_with_format(
                        i as u32 + 1,
                        offset,
                        main_trace.get(i, j).as_canonical_u32() as f64,
                        &format,
                    )?;
                    offset += 1;
                }
                // Blank column
                offset += 1;
            }

            let preprocessed_row = preprocessed_trace
                .as_ref()
                .map(|preprocessed| {
                    let row = preprocessed.row_slice(i);
                    let row: &[_] = (*row).borrow();
                    row.to_vec()
                })
                .unwrap_or_default();
            let main_row = main_trace
                .as_ref()
                .map(|main| {
                    let row = main.row_slice(i);
                    let row: &[_] = (*row).borrow();
                    row.to_vec()
                })
                .unwrap_or_default();

            for (j, (interaction, _)) in interactions.iter().enumerate() {
                let count = interaction
                    .count
                    .apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                let format = {
                    if entries.contains(&TraceEntry::VirtualColumnCount {
                        row: i,
                        interaction: j,
                    }) {
                        Format::new().set_background_color(Color::Red)
                    } else {
                        Format::new()
                    }
                };
                ws.write_number_with_format(
                    i as u32 + 1,
                    offset,
                    count.as_canonical_u32() as f64,
                    &format,
                )?;
                offset += 1;
                for (k, field) in interaction.fields.iter().enumerate() {
                    let val = field.apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                    let format = {
                        if entries.contains(&TraceEntry::VirtualColumnField {
                            row: i,
                            interaction: j,
                            field: k,
                        }) {
                            Format::new().set_background_color(Color::Red)
                        } else {
                            Format::new()
                        }
                    };

                    ws.write_number_with_format(
                        i as u32 + 1,
                        offset,
                        val.as_canonical_u32() as f64,
                        &format,
                    )?;
                    offset += 1;
                }
                // Blank column
                offset += 1;
            }
        }

        Ok(())
    }
}

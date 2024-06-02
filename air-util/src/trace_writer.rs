use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

use core::error::Error;
use core::{borrow::Borrow, iter::once};

use p3_field::{ExtensionField, Field, PrimeField32};
use p3_interaction::Interaction;
use p3_matrix::{dense::RowMajorMatrixView, Matrix};
use rust_xlsxwriter::{Color, Format, Worksheet};

use crate::util::TraceEntry;

pub trait TraceWriter<F: Field, EF: ExtensionField<F>> {
    fn preprocessed_headers() -> Vec<String>;
    fn main_headers() -> Vec<String>;

    fn write_traces_to_worksheet(
        &self,
        ws: &mut Worksheet,
        preprocessed_trace: &Option<RowMajorMatrixView<F>>,
        main_trace: &Option<RowMajorMatrixView<F>>,
        receives: Vec<Interaction<F>>,
        sends: Vec<Interaction<F>>,
        entries: BTreeSet<TraceEntry>,
    ) -> Result<(), Box<dyn Error>>
    where
        F: PrimeField32,
    {
        let perprocessed_headers = Self::preprocessed_headers();
        let main_headers = Self::main_headers();

        let receive_headers: Vec<_> = receives
            .iter()
            .enumerate()
            .flat_map(|(i, interaction)| {
                once("".to_string())
                    .chain(once("count".to_string()))
                    .chain(
                        interaction
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(j, _)| format!("receives[{}][{}]", i, j)),
                    )
                    .collect::<Vec<_>>()
            })
            .collect();
        let send_headers: Vec<_> = sends
            .iter()
            .enumerate()
            .flat_map(|(i, interaction)| {
                once("".to_string())
                    .chain(once("count".to_string()))
                    .chain(
                        interaction
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(j, _)| format!("sends[{}][{}]", i, j)),
                    )
                    .collect::<Vec<_>>()
            })
            .collect();

        let headers: Vec<_> = perprocessed_headers
            .iter()
            .chain(main_headers.iter())
            .chain(receive_headers.iter())
            .chain(send_headers.iter())
            .collect();
        ws.write_row(0, 0, headers)?;

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
                        offset + j as u16,
                        preprocessed_trace.get(i, j).as_canonical_u32() as f64,
                        &format,
                    )?;
                }
                offset += preprocessed_trace.width() as u16;
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
                        offset + j as u16,
                        main_trace.get(i, j).as_canonical_u32() as f64,
                        &format,
                    )?;
                }
                offset += main_trace.width() as u16;
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

            for interaction in receives.iter() {
                // Blank column
                offset += 1;
                let count = interaction
                    .count
                    .apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                ws.write_number(i as u32 + 1, offset, count.as_canonical_u32() as f64)?;
                offset += 1;
                for field in interaction.fields.iter() {
                    let val = field.apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                    ws.write_number(i as u32 + 1, offset, val.as_canonical_u32() as f64)?;
                    offset += 1;
                }
            }
            for interaction in sends.iter() {
                // Blank column
                offset += 1;
                let count = interaction
                    .count
                    .apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                ws.write_number(i as u32 + 1, offset, count.as_canonical_u32() as f64)?;
                offset += 1;
                for field in interaction.fields.iter() {
                    let val = field.apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                    ws.write_number(i as u32 + 1, offset, val.as_canonical_u32() as f64)?;
                    offset += 1;
                }
            }
        }

        Ok(())
    }
}

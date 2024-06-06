use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::error::Error;

use p3_field::{ExtensionField, PrimeField32};
use p3_interaction::{Interaction, InteractionType};
use p3_matrix::{dense::RowMajorMatrixView, Matrix};
use rust_xlsxwriter::{Color, Format, Worksheet};

use crate::folders::EntriesLog;
use crate::util::TraceEntry;

fn generate_format<T: Copy + Ord>(
    header_format: &mut Format,
    entries: &EntriesLog<T>,
    entry: T,
) -> Format {
    let failing = entries.failing.contains(&entry);
    let constrained = entries.constrained.contains(&entry);
    match (failing, constrained) {
        (true, _) => {
            *header_format = Format::new().set_background_color(Color::Red);
            Format::new().set_background_color(Color::Red)
        }
        (false, true) => Format::new(),
        (_, _) => {
            *header_format = Format::new().set_background_color(Color::Yellow);
            Format::new().set_background_color(Color::Yellow)
        }
    }
}

pub trait AirLogger {
    fn preprocessed_headers(&self) -> Vec<String> {
        vec![]
    }

    fn main_headers(&self) -> Vec<String>;

    #[cfg(feature = "schema")]
    fn preprocessed_headers_and_types(&self) -> Vec<(String, String, ops::Range<usize>)> {
        vec![]
    }

    #[cfg(feature = "schema")]
    fn main_headers_and_types(&self) -> Vec<(String, String, ops::Range<usize>)>;

    fn write_traces_to_worksheet<F, EF>(
        &self,
        ws: &mut Worksheet,
        preprocessed_trace: &Option<RowMajorMatrixView<F>>,
        main_trace: &Option<RowMajorMatrixView<F>>,
        interactions: Vec<(Interaction<F>, InteractionType)>,
        entries: EntriesLog<TraceEntry>,
    ) -> Result<(), Box<dyn Error>>
    where
        F: PrimeField32,
        EF: ExtensionField<F>,
    {
        let mut headers = vec![];

        let preprocessed_headers = self.preprocessed_headers();
        if !preprocessed_headers.is_empty() {
            for header in preprocessed_headers.into_iter() {
                headers.push(header);
            }
            // Blank column
            headers.push(String::new());
        }

        let main_headers = self.main_headers();
        if !main_headers.is_empty() {
            // TODO: Yellow if not all rows are constrained
            for header in main_headers.into_iter() {
                headers.push(header);
            }
            // Blank column
            headers.push(String::new());
        }

        let mut num_receives = 0;
        let mut num_sends = 0;
        for (interaction, ty) in interactions.iter() {
            let (ty, ty_offset) = match ty {
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

            let header = format!("{prefix}.count");
            headers.push(header);

            for k in 0..interaction.fields.len() {
                let header = format!("{prefix}[{k}]");
                headers.push(header);
            }
            // Blank column
            headers.push(String::new());
        }

        let mut header_format = headers.iter().map(|_| Format::new()).collect::<Vec<_>>();

        let preprocessed_height = preprocessed_trace.as_ref().map_or(0, |t| t.height());
        let main_height = main_trace.as_ref().map_or(0, |t| t.height());
        let height = preprocessed_height.max(main_height);
        for i in 0..height {
            let mut offset = 0;
            if let Some(preprocessed_trace) = preprocessed_trace {
                for j in 0..preprocessed_trace.width() {
                    let format = generate_format(
                        &mut header_format[offset],
                        &entries,
                        TraceEntry::Preprocessed { row: i, col: j },
                    );
                    ws.write_number_with_format(
                        i as u32 + 1,
                        offset as u16,
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
                    let format = generate_format(
                        &mut header_format[offset],
                        &entries,
                        TraceEntry::Main { row: i, col: j },
                    );
                    ws.write_number_with_format(
                        i as u32 + 1,
                        offset as u16,
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
                let format = generate_format(
                    &mut header_format[offset],
                    &entries,
                    TraceEntry::VirtualColumnCount {
                        row: i,
                        interaction: j,
                    },
                );
                ws.write_number_with_format(
                    i as u32 + 1,
                    offset as u16,
                    count.as_canonical_u32() as f64,
                    &format,
                )?;
                offset += 1;
                for (k, field) in interaction.fields.iter().enumerate() {
                    let val = field.apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                    let format = generate_format(
                        &mut header_format[offset],
                        &entries,
                        TraceEntry::VirtualColumnField {
                            row: i,
                            interaction: j,
                            field: k,
                        },
                    );

                    ws.write_number_with_format(
                        i as u32 + 1,
                        offset as u16,
                        val.as_canonical_u32() as f64,
                        &format,
                    )?;
                    offset += 1;
                }
                // Blank column
                offset += 1;
            }
        }

        for (j, (header, format)) in headers.iter().zip(header_format.iter()).enumerate() {
            ws.write_string_with_format(0, j as u16, header, &format)?;
        }

        Ok(())
    }
}

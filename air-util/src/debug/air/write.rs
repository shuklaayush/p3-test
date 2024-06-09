use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::error::Error;

use p3_field::PrimeField32;
use p3_matrix::{dense::RowMajorMatrixView, Matrix};
use rust_xlsxwriter::{Format, Worksheet};

use crate::debug::generate_format;
use crate::folders::EntriesLog;
use crate::util::TraceEntry;

pub fn write_traces_to_worksheet<F>(
    ws: &mut Worksheet,
    preprocessed_headers: Vec<String>,
    main_headers: Vec<String>,
    preprocessed_trace: &Option<RowMajorMatrixView<F>>,
    main_trace: &Option<RowMajorMatrixView<F>>,
    entries: EntriesLog<TraceEntry>,
) -> Result<(), Box<dyn Error>>
where
    F: PrimeField32,
{
    let preprocessed_width = preprocessed_trace.as_ref().map_or(0, |t| t.width() + 1);
    let main_width = main_trace.as_ref().map_or(0, |t| t.width() + 1);
    let total_width = preprocessed_width + main_width;
    let mut headers = vec![String::new(); total_width];

    let mut offset = 0;
    if !preprocessed_headers.is_empty() {
        for header in preprocessed_headers.into_iter() {
            headers[offset] = header;
            offset += 1;
        }
        // Blank column
        offset += 1;
    }

    if !main_headers.is_empty() {
        for header in main_headers.into_iter() {
            headers[offset] = header;
            offset += 1;
        }
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
        }
    }

    for (j, (header, format)) in headers.iter().zip(header_format.iter()).enumerate() {
        ws.write_string_with_format(0, j as u16, header, format)?;
    }

    Ok(())
}

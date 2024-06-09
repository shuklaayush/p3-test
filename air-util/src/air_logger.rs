use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::error::Error;

use p3_field::{ExtensionField, PrimeField32};
use p3_interaction::{Interaction, InteractionType};
use p3_matrix::dense::RowMajorMatrixView;
use rust_xlsxwriter::Worksheet;

use crate::debug::rap::write_traces_to_worksheet;
use crate::folders::EntriesLog;
use crate::util::TraceEntry;

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
        write_traces_to_worksheet(
            ws,
            self.preprocessed_headers(),
            self.main_headers(),
            preprocessed_trace,
            main_trace,
            interactions,
            entries,
        )
    }
}

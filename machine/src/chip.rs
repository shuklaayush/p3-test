use p3_field::{ExtensionField, Field};
use p3_matrix::dense::RowMajorMatrix;

#[cfg(feature = "debug-trace")]
use p3_field::PrimeField32;
#[cfg(feature = "debug-trace")]
use rust_xlsxwriter::Worksheet;
#[cfg(feature = "debug-trace")]
use std::error::Error;

use p3_rap::generate_permutation_trace;
use p3_rap::Interaction;

pub trait Chip<F: Field> {
    fn generate_trace(&self) -> RowMajorMatrix<F>;

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String>;
}

pub trait RapChip<F: Field, EF: ExtensionField<F>>: Chip<F> {
    /// Generate the permutation trace for a chip with the provided machine.
    /// This is called only after `generate_trace` has been called on all chips.
    fn generate_permutation_trace(
        &self,
        preprocessed: &Option<RowMajorMatrix<F>>,
        main: &RowMajorMatrix<F>,
        sends: &[Interaction<F>],
        receives: &[Interaction<F>],
        random_elements: Vec<EF>,
    ) -> Option<RowMajorMatrix<EF>> {
        generate_permutation_trace(preprocessed, main, sends, receives, random_elements)
    }

    #[cfg(feature = "debug-trace")]
    fn write_traces_to_worksheet(
        &self,
        ws: &mut Worksheet,
        preprocessed_trace: &Option<RowMajorMatrix<F>>,
        main_trace: &RowMajorMatrix<F>,
        perm_trace: &Option<RowMajorMatrix<EF>>,
        num_sends: usize,
        num_receives: usize,
    ) -> Result<(), Box<dyn Error>>
    where
        F: PrimeField32,
    {
        use std::iter::once;

        use itertools::Itertools;
        use p3_matrix::Matrix;

        let perprocessed_headers = (0..preprocessed_trace.as_ref().map_or(0, |t| t.width()))
            .map(|i| format!("preprocessed[{}]", i))
            .collect_vec();

        let main_headers = self.main_headers();

        // TODO: Change name to bus name
        let h1 = (0..num_sends)
            .enumerate()
            .map(|(i, _)| format!("sends[{}]", i))
            .collect_vec();
        let h2 = (0..num_receives)
            .enumerate()
            .map(|(i, _)| format!("receives[{}]", i))
            .collect_vec();
        let perm_headers = h1
            .into_iter()
            .chain(h2)
            .chain(once("cumulative_sum".to_string()))
            .collect_vec();

        let headers = perprocessed_headers
            .iter()
            .chain(main_headers.iter())
            .chain(perm_headers.iter())
            .collect_vec();
        ws.write_row(0, 0, headers)?;

        let preprocessed_height = preprocessed_trace.as_ref().map_or(0, |t| t.height());
        let main_height = main_trace.height();
        let perm_height = perm_trace.as_ref().map_or(0, |t| t.height());
        let max_height = preprocessed_height.max(main_height).max(perm_height);

        for i in 0..max_height {
            let mut offset = 0;
            if let Some(preprocessed_trace) = preprocessed_trace {
                for j in 0..preprocessed_trace.width() {
                    ws.write_number(
                        i as u32 + 1,
                        offset + j as u16,
                        preprocessed_trace.get(i, j).as_canonical_u32() as f64,
                    )?;
                }
                offset += preprocessed_trace.width() as u16;
            }

            for j in 0..main_trace.width() {
                ws.write_number(
                    i as u32 + 1,
                    offset + j as u16,
                    main_trace.get(i, j).as_canonical_u32() as f64,
                )?;
            }
            offset += main_trace.width() as u16;

            if let Some(perm_trace) = perm_trace {
                for j in 0..perm_trace.width() {
                    ws.write(
                        i as u32 + 1,
                        offset + j as u16,
                        perm_trace.get(i, j).to_string(),
                    )?;
                }
            }
        }

        Ok(())
    }
}

// pub trait Chip<SC: StarkGenericConfig>:
//     for<'a> Rap<ProverConstraintFolder<'a, SC>>
//     + for<'a> Rap<VerifierConstraintFolder<'a, SC>>
//     + for<'a> Rap<DebugConstraintBuilder<'a, SC>>

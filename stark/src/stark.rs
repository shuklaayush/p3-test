use p3_field::{ExtensionField, Field};
use p3_interaction::InteractionType;
use p3_interaction::NUM_PERM_CHALLENGES;
use p3_matrix::dense::RowMajorMatrix;

#[cfg(feature = "debug-trace")]
use p3_field::PrimeField32;
#[cfg(feature = "debug-trace")]
use rust_xlsxwriter::Worksheet;
#[cfg(feature = "debug-trace")]
use std::error::Error;

use p3_interaction::generate_permutation_trace;
use p3_interaction::Interaction;

pub trait Stark<F: Field> {
    fn generate_trace(&self) -> RowMajorMatrix<F>;

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String>;
}

pub trait InteractionStark<F: Field, EF: ExtensionField<F>>: Stark<F> {
    fn generate_permutation_trace(
        &self,
        preprocessed: &Option<RowMajorMatrix<F>>,
        main: &Option<RowMajorMatrix<F>>,
        interactions: &[(Interaction<F>, InteractionType)],
        random_elements: [EF; NUM_PERM_CHALLENGES],
    ) -> Option<RowMajorMatrix<EF>> {
        generate_permutation_trace(preprocessed, main, interactions, random_elements)
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
        use p3_matrix::Matrix;
        use std::iter::once;

        let perprocessed_headers: Vec<_> =
            (0..preprocessed_trace.as_ref().map_or(0, |t| t.width()))
                .map(|i| format!("preprocessed[{}]", i))
                .collect();

        let main_headers = self.main_headers();

        // TODO: Change name to bus name
        let h1: Vec<_> = (0..num_sends)
            .enumerate()
            .map(|(i, _)| format!("sends[{}]", i))
            .collect();
        let h2: Vec<_> = (0..num_receives)
            .enumerate()
            .map(|(i, _)| format!("receives[{}]", i))
            .collect();
        let perm_headers: Vec<_> = h1
            .into_iter()
            .chain(h2)
            .chain(once("cumulative_sum".to_string()))
            .collect();

        let headers: Vec<_> = perprocessed_headers
            .iter()
            .chain(main_headers.iter())
            .chain(perm_headers.iter())
            .collect();
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

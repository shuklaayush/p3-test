use p3_air::Air;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, Val};

#[cfg(feature = "debug-trace")]
use p3_field::PrimeField64;
#[cfg(feature = "debug-trace")]
use rust_xlsxwriter::Worksheet;
#[cfg(feature = "debug-trace")]
use std::error::Error;

use crate::debug_builder::DebugConstraintBuilder;
use crate::folder::{ProverConstraintFolder, VerifierConstraintFolder};
use crate::interaction::{Interaction, InteractionType};

pub trait Chip<F: Field> {
    fn generate_trace(&self) -> RowMajorMatrix<F>;

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn all_interactions(&self) -> Vec<(Interaction<F>, InteractionType)> {
        let mut interactions: Vec<(Interaction<F>, InteractionType)> = vec![];
        interactions.extend(self.sends().into_iter().map(|i| (i, InteractionType::Send)));
        interactions.extend(
            self.receives()
                .into_iter()
                .map(|i| (i, InteractionType::Receive)),
        );
        interactions
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String>;

    #[cfg(feature = "debug-trace")]
    fn write_traces_to_worksheet<E: Field>(
        &self,
        ws: &mut Worksheet,
        preprocessed_trace: &Option<RowMajorMatrix<F>>,
        main_trace: &RowMajorMatrix<F>,
        perm_trace: &RowMajorMatrix<E>,
    ) -> Result<(), Box<dyn Error>>
    where
        F: PrimeField64,
    {
        use std::iter::once;

        use itertools::Itertools;
        use p3_matrix::Matrix;

        let perprocessed_headers = (0..preprocessed_trace.as_ref().map_or(0, |t| t.width()))
            .map(|i| format!("preprocessed[{}]", i))
            .collect_vec();

        let main_headers = self.main_headers();

        let sends: Vec<Interaction<F>> = self.sends();
        let receives: Vec<Interaction<F>> = self.receives();

        // TODO: Change name to bus name
        let h1 = sends
            .iter()
            .enumerate()
            .map(|(i, _)| format!("sends[{}]", i))
            .collect_vec();
        let h2 = receives
            .iter()
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

        let max_height = main_trace
            .height()
            .max(perm_trace.height())
            .max(preprocessed_trace.as_ref().map_or(0, |t| t.height()));
        for i in 0..max_height {
            let mut offset = 0;
            if let Some(preprocessed_trace) = preprocessed_trace {
                for j in 0..preprocessed_trace.width() {
                    ws.write_number(
                        i as u32 + 1,
                        offset + j as u16,
                        preprocessed_trace.get(i, j).as_canonical_u64() as f64,
                    )?;
                }
                offset += preprocessed_trace.width() as u16;
            }
            for j in 0..main_trace.width() {
                ws.write_number(
                    i as u32 + 1,
                    offset + j as u16,
                    main_trace.get(i, j).as_canonical_u64() as f64,
                )?;
            }
            offset += main_trace.width() as u16;
            for j in 0..perm_trace.width() {
                ws.write(
                    i as u32 + 1,
                    offset + j as u16,
                    perm_trace.get(i, j).to_string(),
                )?;
            }
        }

        Ok(())
    }
}

pub trait MachineChip<SC: StarkGenericConfig>:
    Chip<Val<SC>>
    + for<'a> Air<ProverConstraintFolder<'a, SC>>
    + for<'a> Air<VerifierConstraintFolder<'a, SC>>
    + for<'a> Air<DebugConstraintBuilder<'a, SC>>
{
    fn trace_width(&self) -> usize {
        self.width()
    }
}

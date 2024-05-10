use itertools::Itertools;
use p3_field::{ExtensionField, Field};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use std::borrow::Borrow;

#[cfg(feature = "debug-trace")]
use p3_field::PrimeField32;
#[cfg(feature = "debug-trace")]
use rust_xlsxwriter::Worksheet;
#[cfg(feature = "debug-trace")]
use std::error::Error;

use crate::rap::interaction::{Interaction, InteractionType};
use crate::rap::permutation_air::{generate_rlc_elements, reduce_row};
use crate::util::batch_multiplicative_inverse_allowing_zero;

pub trait Chip<F: Field> {
    fn generate_trace(&self) -> RowMajorMatrix<F>;

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String>;
}

pub trait PermutationChip<F: Field, EF: ExtensionField<F>> {
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
        let interactions = sends
            .into_iter()
            .map(|i| (i, InteractionType::Send))
            .chain(receives.into_iter().map(|i| (i, InteractionType::Receive)))
            .collect_vec();
        if interactions.is_empty() {
            return None;
        }

        let alphas = generate_rlc_elements(sends, receives, random_elements[0]);
        let betas = random_elements[1].powers();

        // Compute the reciprocal columns
        //
        // Row: | q_1 | q_2 | q_3 | ... | q_n | \phi |
        // * q_i = \frac{1}{\alpha^i + \sum_j \beta^j * f_{i,j}}
        // * f_{i,j} is the jth main trace column for the ith interaction
        // * \phi is the running sum
        //
        // Note: We can optimize this by combining several reciprocal columns into one (the
        // number is subject to a target constraint degree).
        let perm_width = interactions.len() + 1;
        let mut perm_values = Vec::with_capacity(main.height() * perm_width);

        for (n, main_row) in main.rows().enumerate() {
            let main_row = main_row.collect_vec();

            let mut row = vec![EF::zero(); perm_width];
            for (m, (interaction, _)) in interactions.iter().enumerate() {
                let alpha_m = alphas[interaction.argument_index];
                let preprocessed_row = preprocessed
                    .as_ref()
                    .map(|preprocessed| {
                        let row = preprocessed.row_slice(n);
                        let row: &[_] = (*row).borrow();
                        row.to_vec()
                    })
                    .unwrap_or_default();
                row[m] = reduce_row(
                    main_row.as_slice(),
                    preprocessed_row.as_slice(),
                    &interaction.fields,
                    alpha_m,
                    betas.clone(),
                );
            }
            perm_values.extend(row);
        }
        // TODO: Switch to batch_multiplicative_inverse (not allowing zero)?
        // Zero should be vanishingly unlikely if properly randomized?
        let perm_values = batch_multiplicative_inverse_allowing_zero(perm_values);
        let mut perm = RowMajorMatrix::new(perm_values, perm_width);

        // Compute the running sum column
        let mut phi = vec![EF::zero(); perm.height()];
        for (n, (main_row, perm_row)) in main.rows().zip(perm.rows()).enumerate() {
            let main_row = main_row.collect_vec();
            let perm_row = perm_row.collect_vec();

            if n > 0 {
                phi[n] = phi[n - 1];
            }
            let preprocessed_row = preprocessed
                .as_ref()
                .map(|preprocessed| {
                    let row = preprocessed.row_slice(n);
                    let row: &[_] = (*row).borrow();
                    row.to_vec()
                })
                .unwrap_or_default();
            for (m, (interaction, interaction_type)) in interactions.iter().enumerate() {
                let mult = interaction
                    .count
                    .apply::<F, F>(preprocessed_row.as_slice(), main_row.as_slice());
                match interaction_type {
                    InteractionType::Send => {
                        phi[n] += perm_row[m] * mult;
                    }
                    InteractionType::Receive => {
                        phi[n] -= perm_row[m] * mult;
                    }
                }
            }
        }

        for (n, row) in perm.as_view_mut().rows_mut().enumerate() {
            *row.last_mut().unwrap() = phi[n];
        }

        Some(perm)
    }
}

pub trait RapChip<F: Field, EF: ExtensionField<F>>: Chip<F> + PermutationChip<F, EF> {
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

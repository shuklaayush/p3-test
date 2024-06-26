use alloc::vec::Vec;

use itertools::Itertools;
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
#[cfg(feature = "schema")]
use p3_field::Field;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, Val};
use tracing::instrument;

use p3_air_util::folders::rap::{
    DebugConstraintBuilder, ProverConstraintFolder, SymbolicAirBuilder, TrackingConstraintBuilder,
    VerifierConstraintFolder,
};
use p3_air_util::proof::Commitments;
#[cfg(feature = "schema")]
use p3_interaction::InteractionAir;
use p3_interaction::{Bus, Rap, NUM_PERM_CHALLENGES};

#[cfg(debug_assertions)]
use crate::trace::MachineTraceChecker;
#[cfg(feature = "air-logger")]
use crate::trace::MachineTraceDebugger;
use crate::{
    chip::Chip,
    error::VerificationError,
    proof::{
        MachineProof, ProverPreprocessedData, ProvingKey, VerifierPreprocessedData, VerifyingKey,
    },
    trace::{
        MachineTrace, MachineTraceBuilder, MachineTraceCommiter, MachineTraceConstraintVerifier,
        MachineTraceLoader, MachineTraceOpener, MachineTraceOpening, MachineTraceOpeningBuilder,
        MachineTraceOpeningLoader, MachineTraceOpeningVerifier,
    },
};

pub trait Machine {
    type Chip: Chip;

    type Bus: Bus;

    fn chips(&self) -> Vec<Self::Chip>;

    fn setup<'a, SC>(&self, config: &'a SC) -> (ProvingKey<SC>, VerifyingKey<SC>)
    where
        SC: StarkGenericConfig,
        Self::Chip: for<'b> Rap<ProverConstraintFolder<'b, SC>>
            + for<'b> Rap<VerifierConstraintFolder<'b, SC>>
            + for<'b> Rap<SymbolicAirBuilder<Val<SC>>>
            + for<'b> Rap<DebugConstraintBuilder<'b, Val<SC>, SC::Challenge>>,
    {
        let pcs = config.pcs();
        let chips = self.chips();
        let mut trace: MachineTrace<SC, _> = MachineTraceBuilder::new(chips.as_slice());

        // 1. Generate and commit to preprocessed traces
        tracing::info_span!("generate preprocessed traces")
            .in_scope(|| trace.generate_preprocessed(pcs));

        let traces = trace
            .iter()
            .map(|chip_trace| {
                chip_trace
                    .preprocessed
                    .as_ref()
                    .map(|preprocessed| preprocessed.trace.value.clone())
            })
            .collect();
        let indexed_degrees: Vec<(usize, usize)> = trace
            .iter()
            .enumerate()
            .flat_map(|(i, chip_trace)| {
                chip_trace
                    .preprocessed
                    .as_ref()
                    .map(|trace| (i, trace.trace.domain.size()))
            })
            .collect();

        let mut prover_data = ProverPreprocessedData {
            traces,
            commitment: None,
            data: None,
        };
        let verifier_data = if let (Some(commit), Some(data)) = trace.commit_preprocessed(pcs) {
            prover_data.commitment = Some(commit.clone());
            prover_data.data = Some(data);

            Some(VerifierPreprocessedData {
                commitment: commit,
                degrees: indexed_degrees,
            })
        } else {
            None
        };

        let vk = VerifyingKey {
            preprocessed: verifier_data,
        };
        let pk = ProvingKey {
            preprocessed: prover_data,
        };

        (pk, vk)
    }

    fn prove<'a, SC>(
        &self,
        config: &'a SC,
        challenger: &mut SC::Challenger,
        pk: &'a ProvingKey<SC>,
        main_traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
        // TODO: Change to 2d vector?
        public_values: &'a [Val<SC>],
    ) -> MachineProof<SC>
    where
        SC: StarkGenericConfig,
        Self::Chip: for<'b> Rap<ProverConstraintFolder<'b, SC>>
            + for<'b> Rap<VerifierConstraintFolder<'b, SC>>
            + for<'b> Rap<SymbolicAirBuilder<Val<SC>>>
            + for<'b> Rap<DebugConstraintBuilder<'b, Val<SC>, SC::Challenge>>
            // TODO: Put behind air-logger feature
            + for<'b> Rap<TrackingConstraintBuilder<'b, Val<SC>, SC::Challenge>>,
        Val<SC>: PrimeField32,
    {
        // TODO: Use fixed size array instead of Vecs
        let chips = self.chips();
        assert_eq!(main_traces.len(), chips.len(), "Length mismatch");

        let pcs = config.pcs();

        // 1. Observe public values
        challenger.observe_slice(public_values);

        let mut trace: MachineTrace<SC, _> = MachineTraceBuilder::new(&chips);

        // 2. Observe preprocessed commitment
        tracing::info_span!("load preprocessed traces")
            .in_scope(|| trace.load_preprocessed(pcs, pk.preprocessed.traces.as_slice()));
        if let Some(commit) = &pk.preprocessed.commitment {
            challenger.observe(commit.clone());
        }

        // 3. Generate and commit to main trace
        tracing::info_span!("load main traces").in_scope(|| trace.load_main(pcs, main_traces));
        let (main_commit, main_data) =
            tracing::info_span!("commit to main traces").in_scope(|| trace.commit_main(pcs));
        if let Some(main_commit) = &main_commit {
            challenger.observe(main_commit.clone());
        }

        // 4. Sample permutation challenges
        let perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES] = (0..NUM_PERM_CHALLENGES)
            .map(|_| challenger.sample_ext_element::<SC::Challenge>())
            .collect_vec()
            .try_into()
            .unwrap();

        // 5. Generate and commit to permutation trace
        tracing::info_span!("generate permutation traces")
            .in_scope(|| trace.generate_permutation(pcs, perm_challenges));
        let (permutation_commit, permutation_data) =
            tracing::info_span!("commit to permutation traces")
                .in_scope(|| trace.commit_permutation(pcs));
        if let Some(permutation_commit) = &permutation_commit {
            challenger.observe(permutation_commit.clone());
        }
        let alpha: SC::Challenge = challenger.sample_ext_element();

        #[cfg(feature = "air-logger")]
        let _ = tracing::info_span!("writing traces to file")
            .in_scope(|| trace.write_traces_to_file("trace.xlsx", perm_challenges));

        // Verify constraints
        #[cfg(debug_assertions)]
        tracing::info_span!("checking constraints")
            .in_scope(|| trace.check_constraints::<Self::Bus>(perm_challenges, &[]));

        // 6. Generate and commit to quotient traces
        tracing::info_span!("generate quotient trace").in_scope(|| {
            trace.generate_quotient(
                pcs,
                &pk.preprocessed.data,
                &main_data,
                &permutation_data,
                perm_challenges,
                alpha,
                public_values,
            )
        });
        // TODO: Panic if this is None
        let (quotient_commit, quotient_data) = tracing::info_span!("commit to quotient chunks")
            .in_scope(|| trace.commit_quotient(pcs));
        if let Some(quotient_commit) = &quotient_commit {
            challenger.observe(quotient_commit.clone());
        }

        let commitments = Commitments {
            main: main_commit,
            permutation: permutation_commit,
            quotient_chunks: quotient_commit,
        };

        // 7. Sample OOD point and generate opening proof
        let zeta: SC::Challenge = challenger.sample_ext_element();
        let rounds = trace.generate_rounds(
            zeta,
            &pk.preprocessed.data,
            &main_data,
            &permutation_data,
            &quotient_data,
        );
        let (opening_values, opening_proof) = pcs.open(rounds, challenger);

        // Unflatten quotient openings
        let opening_values = trace.unflatten_openings(
            opening_values,
            &pk.preprocessed.data,
            &main_data,
            &permutation_data,
            &quotient_data,
        );

        let chip_proofs = trace.generate_proofs(opening_values);

        MachineProof {
            commitments,
            opening_proof,
            chip_proofs,
        }
    }

    #[instrument(skip_all)]
    fn verify<'a, SC>(
        &self,
        config: &'a SC,
        challenger: &'a mut SC::Challenger,
        vk: &'a VerifyingKey<SC>,
        proof: &MachineProof<SC>,
        public_values: &'a [Val<SC>],
    ) -> Result<(), VerificationError>
    where
        SC: StarkGenericConfig,
        Val<SC>: PrimeField32,
        Self::Chip: for<'b> Rap<VerifierConstraintFolder<'b, SC>>
            + for<'b> Rap<SymbolicAirBuilder<Val<SC>>>,
    {
        let chips = self.chips();
        let pcs = config.pcs();

        let mut trace: MachineTraceOpening<SC, _> = MachineTraceOpeningBuilder::new(&chips);

        let MachineProof {
            commitments,
            opening_proof,
            chip_proofs,
        } = proof;

        let mut preprocessed_degrees = (0..trace.len()).map(|_| 0usize).collect_vec();
        if let Some(preprocessed) = &vk.preprocessed {
            for (i, degree) in preprocessed.degrees.iter() {
                preprocessed_degrees[*i] = *degree;
            }
        }
        // TODO: Avoid clone
        trace.load_openings(pcs, chip_proofs.clone(), preprocessed_degrees);

        // Verify proof shape
        trace.verify_shapes()?;

        // Observe commitments
        if let Some(preprocessed) = &vk.preprocessed {
            challenger.observe(preprocessed.commitment.clone());
        }
        if let Some(main) = &commitments.main {
            challenger.observe(main.clone());
        }
        let perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES] = (0..NUM_PERM_CHALLENGES)
            .map(|_| challenger.sample_ext_element::<SC::Challenge>())
            .collect_vec()
            .try_into()
            .unwrap();
        if let Some(permutation) = &commitments.permutation {
            challenger.observe(permutation.clone());
        }
        let alpha = challenger.sample_ext_element::<SC::Challenge>();
        if let Some(quotient_chunks) = &commitments.quotient_chunks {
            challenger.observe(quotient_chunks.clone());
        }

        let zeta: SC::Challenge = challenger.sample_ext_element();

        // TODO: Remove clone
        let rounds = trace.generate_rounds(
            zeta,
            &vk.preprocessed
                .as_ref()
                .map(|preprocessed| preprocessed.commitment.clone()),
            &commitments.main,
            &commitments.permutation,
            &commitments.quotient_chunks,
        );

        pcs.verify(rounds, opening_proof, challenger)
            .map_err(|_| VerificationError::InvalidOpeningArgument)?;

        // Verify constraints at zeta
        trace.verify_constraints(zeta, alpha, perm_challenges, public_values)?;

        // Verify cumulative sum adds to zero
        trace.verify_cumulative_sums()?;

        Ok(())
    }

    #[cfg(feature = "schema")]
    fn write_schema_to_file<F>(&self, path: &str)
    where
        F: Field,
        Self::Chip: InteractionAir<F>,
    {
        use alloc::collections::BTreeMap;
        use alloc::format;
        use alloc::vec;
        use alloc::vec::Vec;
        use core::iter::once;
        use p3_air::PairCol;
        use p3_air_util::AirLogger;
        use p3_interaction::InteractionType;
        use std::fs::File;
        use std::io::{BufWriter, Write};

        let chips = self.chips();
        let f = File::create(path).expect("Unable to create file");
        let mut f = BufWriter::new(f);
        let mut bus_lengths = BTreeMap::new();
        let mut interaction_rows = vec![];
        for chip in chips.iter() {
            let mut chip_rows = BTreeMap::new();
            let headers_and_types = chip
                .preprocessed_headers_and_types()
                .into_iter()
                .chain(chip.main_headers_and_types().into_iter())
                .collect::<Vec<_>>();
            let preprocessed_headers = chip
                .preprocessed_headers_and_types()
                .into_iter()
                .flat_map(|(header, _, header_range)| header_range.map(move |_| header.clone()))
                .collect::<Vec<_>>();
            let main_headers = chip
                .main_headers_and_types()
                .into_iter()
                .flat_map(|(header, _, header_range)| header_range.map(move |_| header.clone()))
                .collect::<Vec<_>>();
            let body = headers_and_types
                .iter()
                .map(|(header, ty, _)| format!("    \"{}\" {}", header, ty))
                .join("\n");
            let table = format!("Table {} {{\n{}\n}}\n\n", chip, body);
            f.write_all(table.as_bytes()).expect("Unable to write data");

            for (interaction, ty) in chip.all_interactions().iter() {
                let bus = Self::Bus::from(interaction.argument_index);
                bus_lengths
                    .entry(bus as usize)
                    .and_modify(|existing_length| {
                        *existing_length =
                            core::cmp::max(*existing_length, interaction.fields.len());
                    })
                    .or_insert(interaction.fields.len());

                let direction = match ty {
                    InteractionType::Receive => '<',
                    InteractionType::Send => '>',
                };
                for (col, _) in interaction.count.column_weights.iter() {
                    let header = match col {
                        PairCol::Preprocessed(k) => &preprocessed_headers[*k],
                        PairCol::Main(k) => &main_headers[*k],
                    };
                    let row = (
                        format!("Ref: \"{}\".\"{}\"", chip, header),
                        format!("\"{}\".\"count\"\n", bus),
                    );
                    if chip_rows.contains_key(&row) && chip_rows[&row] != direction {
                        chip_rows.insert(row, '-');
                    } else {
                        chip_rows.insert(row, direction);
                    }
                }
                for (j, field) in interaction.fields.iter().enumerate() {
                    for (col, _) in field.column_weights.iter() {
                        let header = match col {
                            PairCol::Preprocessed(k) => &preprocessed_headers[*k],
                            PairCol::Main(k) => &main_headers[*k],
                        };
                        let row = (
                            format!("Ref: \"{}\".\"{}\"", chip, header),
                            format!("\"{}\".\"vc[{}]\"\n", bus, j),
                        );
                        if chip_rows.contains_key(&row) && chip_rows[&row] != direction {
                            chip_rows.insert(row, '-');
                        } else {
                            chip_rows.insert(row, direction);
                        }
                    }
                }
            }
            interaction_rows.push(chip_rows);
        }
        for (i, &length) in bus_lengths.iter() {
            let body = once("    \"count\" Field".to_string())
                .chain((0..length).map(|i| format!("    \"vc[{}]\" Field", i)))
                .join("\n");
            let table = format!("Table {} {{\n{}\n}}\n\n", Bus::from(i), body);
            f.write_all(table.as_bytes()).expect("Unable to write data");
        }
        for chip_rows in interaction_rows.into_iter() {
            for ((prefix, suffix), direction) in chip_rows.into_iter() {
                let row = format!("{} {} {}", prefix, direction, suffix);
                f.write_all(row.as_bytes()).expect("Unable to write data");
            }
            f.write_all("\n".as_bytes()).expect("Unable to write data");
        }
    }
}

use alloc::vec::Vec;

use itertools::Itertools;
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::PrimeField32;
use p3_interaction::NUM_PERM_CHALLENGES;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, Val};
use tracing::instrument;

use p3_air_util::proof::Commitments;

use crate::{
    chip::MachineChip,
    error::VerificationError,
    proof::{
        MachineProof, ProverPreprocessedData, ProvingKey, VerifierPreprocessedData, VerifyingKey,
    },
    trace::{
        MachineTrace, MachineTraceBuilder, MachineTraceChecker, MachineTraceCommiter,
        MachineTraceConstraintVerifier, MachineTraceDebugger, MachineTraceLoader,
        MachineTraceOpener, MachineTraceOpening, MachineTraceOpeningBuilder,
        MachineTraceOpeningLoader, MachineTraceOpeningVerifier,
    },
};

pub trait Machine<'a, SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn chips(&self) -> Vec<C>;

    fn setup(&self, config: &'a SC) -> (ProvingKey<SC>, VerifyingKey<SC>) {
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

    fn prove(
        &self,
        config: &'a SC,
        challenger: &mut SC::Challenger,
        pk: &'a ProvingKey<SC>,
        main_traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
        // TODO: Change to 2d vector?
        public_values: &'a [Val<SC>],
    ) -> MachineProof<SC>
    where
        // TODO: Put behind trace-writer flag
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

        // Verify constraints
        #[cfg(feature = "trace-writer")]
        let _ = trace.write_traces_to_file("trace.xlsx");
        #[cfg(debug_assertions)]
        trace.check_constraints(perm_challenges, &[]);

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
    fn verify(
        &self,
        config: &'a SC,
        challenger: &'a mut SC::Challenger,
        vk: &'a VerifyingKey<SC>,
        proof: &MachineProof<SC>,
        public_values: &'a [Val<SC>],
    ) -> Result<(), VerificationError> {
        let pcs = config.pcs();
        let chips = self.chips();

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
        trace.check_cumulative_sums()?;

        Ok(())
    }
}

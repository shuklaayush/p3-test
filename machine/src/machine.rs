use itertools::Itertools;
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::PrimeField32;
use p3_interaction::NUM_PERM_CHALLENGES;
use p3_matrix::dense::RowMajorMatrix;
use p3_stark::Commitments;
use p3_uni_stark::{StarkGenericConfig, Val};
use tracing::instrument;

use crate::{
    chip::ChipType,
    chips::{
        keccak_permute::KeccakPermuteChip, keccak_sponge::KeccakSpongeChip, memory::MemoryChip,
        merkle_tree::MerkleTreeChip, range_checker::RangeCheckerChip, xor::XorChip,
    },
    error::VerificationError,
    proof::{
        MachineProof, ProverPreprocessedData, ProvingKey, VerifierPreprocessedData, VerifyingKey,
    },
    trace_util::{
        MachineTrace, MachineTraceBuilder, MachineTraceChecker, MachineTraceCommiter,
        MachineTraceConstraintVerifier, MachineTraceDebugger, MachineTraceLoader,
        MachineTraceOpener, MachineTraceOpening, MachineTraceOpeningBuilder,
        MachineTraceOpeningLoader, MachineTraceOpeningVerifier,
    },
};

pub struct Machine {
    keccak_permute_chip: ChipType,
    keccak_sponge_chip: ChipType,
    merkle_tree_chip: ChipType,
    range_chip: ChipType,
    xor_chip: ChipType,
    memory_chip: ChipType,
}

impl Machine {
    pub fn chips(&self) -> Vec<&ChipType> {
        vec![
            &self.keccak_permute_chip,
            &self.keccak_sponge_chip,
            &self.merkle_tree_chip,
            &self.range_chip,
            &self.xor_chip,
            &self.memory_chip,
        ]
    }
}

pub enum MachineBus {
    KeccakPermuteInput = 0,
    KeccakPermuteOutput = 1,
    KeccakPermuteDigest = 2,
    Range8 = 3,
    XorInput = 4,
    XorOutput = 5,
    Memory = 6,
}

impl Machine {
    fn new() -> Self {
        let keccak_permute_chip = KeccakPermuteChip {
            bus_keccak_permute_input: MachineBus::KeccakPermuteInput as usize,
            bus_keccak_permute_output: MachineBus::KeccakPermuteOutput as usize,
            bus_keccak_permute_digest_output: MachineBus::KeccakPermuteDigest as usize,
        };
        let keccak_sponge_chip = KeccakSpongeChip {
            bus_xor_input: MachineBus::XorInput as usize,
            bus_keccak_permute_input: MachineBus::KeccakPermuteInput as usize,
            bus_range_8: MachineBus::Range8 as usize,
            bus_memory: MachineBus::Memory as usize,
            bus_xor_output: MachineBus::XorOutput as usize,
            bus_keccak_permute_output: MachineBus::KeccakPermuteOutput as usize,
        };
        let merkle_tree_chip = MerkleTreeChip {
            bus_keccak_permute_input: MachineBus::KeccakPermuteInput as usize,
            bus_keccak_digest_output: MachineBus::KeccakPermuteDigest as usize,
        };
        let range_chip = RangeCheckerChip {
            bus_range_8: MachineBus::Range8 as usize,
        };
        let xor_chip = XorChip {
            bus_xor_input: MachineBus::XorInput as usize,
            bus_xor_output: MachineBus::XorOutput as usize,
        };
        let memory_chip = MemoryChip {
            bus_memory: MachineBus::Memory as usize,
            bus_range_8: MachineBus::Range8 as usize,
        };

        Self {
            keccak_permute_chip: ChipType::KeccakPermute(keccak_permute_chip),
            keccak_sponge_chip: ChipType::KeccakSponge(keccak_sponge_chip),
            merkle_tree_chip: ChipType::MerkleTree(merkle_tree_chip),
            range_chip: ChipType::Range8(range_chip),
            xor_chip: ChipType::Xor(xor_chip),
            memory_chip: ChipType::Memory(memory_chip),
        }
    }

    fn setup<SC>(&self, config: &SC) -> (ProvingKey<SC>, VerifyingKey<SC>)
    where
        SC: StarkGenericConfig,
    {
        let pcs = config.pcs();
        let chips = self.chips();
        let mut trace: MachineTrace<SC> = MachineTraceBuilder::new(chips);

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

    #[instrument(skip_all)]
    fn prove<SC>(
        &self,
        config: &SC,
        challenger: &mut SC::Challenger,
        pk: &ProvingKey<SC>,
        main_traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
        // TODO: Change to 2d vector?
        public_values: Vec<Val<SC>>,
    ) -> MachineProof<SC>
    where
        SC: StarkGenericConfig,
        Val<SC>: PrimeField32,
        <<SC as StarkGenericConfig>::Pcs as Pcs<
            <SC as StarkGenericConfig>::Challenge,
            <SC as StarkGenericConfig>::Challenger,
        >>::Domain: Send + Sync,
    {
        // TODO: Use fixed size array instead of Vecs
        let chips = self.chips();
        assert_eq!(main_traces.len(), chips.len(), "Length mismatch");

        let pcs = config.pcs();

        // 1. Observe public values
        challenger.observe_slice(&public_values);

        let mut trace: MachineTrace<SC> = MachineTraceBuilder::new(chips);

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
        #[cfg(feature = "debug-trace")]
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
                &public_values,
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
    fn verify<SC>(
        &self,
        config: &SC,
        challenger: &mut SC::Challenger,
        vk: &VerifyingKey<SC>,
        proof: MachineProof<SC>,
        public_values: Vec<Val<SC>>,
    ) -> Result<(), VerificationError>
    where
        SC: StarkGenericConfig,
        Val<SC>: PrimeField32,
    {
        let pcs = config.pcs();
        let chips = self.chips();

        let mut trace: MachineTraceOpening<SC> = MachineTraceOpeningBuilder::new(chips);

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
        trace.load_openings(pcs, chip_proofs, preprocessed_degrees);

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

        pcs.verify(rounds, &opening_proof, challenger)
            .map_err(|_| VerificationError::InvalidOpeningArgument)?;

        // Verify constraints at zeta
        trace.verify_constraints(zeta, alpha, perm_challenges, &public_values)?;

        // Verify cumulative sum adds to zero
        trace.check_cumulative_sums()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{default_challenger, default_config, MyConfig},
        trace::generate_machine_trace,
    };

    use p3_keccak::KeccakF;
    use p3_symmetric::{PseudoCompressionFunction, TruncatedPermutation};
    use rand::{random, thread_rng, Rng};
    use tracing_forest::{util::LevelFilter, ForestLayer};
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

    fn generate_digests(leaf_hashes: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
        let keccak = TruncatedPermutation::new(KeccakF {});
        let mut digests = vec![leaf_hashes.to_vec()];

        while let Some(last_level) = digests.last().cloned() {
            if last_level.len() == 1 {
                break;
            }

            let next_level = last_level
                .chunks_exact(2)
                .map(|chunk| keccak.compress([chunk[0], chunk[1]]))
                .collect();

            digests.push(next_level);
        }

        digests
    }

    #[test]
    fn test_machine_prove() -> Result<(), VerificationError> {
        let env_filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();

        Registry::default()
            .with(env_filter)
            .with(ForestLayer::default())
            .init();

        const NUM_BYTES: usize = 1000;
        let preimage = (0..NUM_BYTES).map(|_| random()).collect_vec();

        const HEIGHT: usize = 8;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32)).map(|_| random()).collect_vec();
        let digests = generate_digests(&leaf_hashes);

        let leaf_index = thread_rng().gen_range(0..leaf_hashes.len());
        let machine = Machine::new();

        let (pk, vk) = machine.setup(&default_config());

        let config = default_config();
        let mut challenger = default_challenger();
        let traces = generate_machine_trace::<MyConfig>(preimage, digests, leaf_index);
        let proof = machine.prove(&config, &mut challenger, &pk, traces, vec![]);

        let mut challenger = default_challenger();
        machine.verify(&config, &mut challenger, &vk, proof, vec![])
    }
}

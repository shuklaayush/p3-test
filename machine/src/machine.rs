use itertools::{izip, Itertools};
use p3_air::BaseAir;
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{AbstractExtensionField, AbstractField, PrimeField32};
use p3_interaction::{InteractionAirBuilder, NUM_PERM_CHALLENGES};
use p3_matrix::dense::RowMajorMatrix;
use p3_stark::{
    symbolic::get_quotient_degree, AdjacentOpenedValues, ChipProof, Commitments, OpenedValues,
};
use p3_uni_stark::{Domain, PackedVal, StarkGenericConfig, Val};
use tracing::instrument;

// #[cfg(feature = "debug-trace")]
// use std::error::Error;

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
        ChipTrace, IndexedTrace, MachineTrace, MachineTraceBuilder, MachineTraceChecker,
        MachineTraceCommiter, MachineTraceLoader, MachineTraceOpener, Trace,
    }, // verify::verify_constraints,
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
        let flat_degrees = trace
            .iter()
            .flat_map(|chip_trace| {
                chip_trace
                    .preprocessed
                    .as_ref()
                    .map(|trace| trace.trace.domain.size())
            })
            .collect::<Vec<usize>>();

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
                degrees: flat_degrees,
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
        pk: &ProvingKey<SC>,
        main_traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
        challenger: &mut SC::Challenger,
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
        let mut trace: MachineTrace<SC> = MachineTraceBuilder::new(chips);

        // 1. Observe preprocessed commitment
        tracing::info_span!("load preprocessed traces")
            .in_scope(|| trace.load_preprocessed(pcs, pk.preprocessed.traces.as_slice()));
        if let Some(commit) = &pk.preprocessed.commitment {
            challenger.observe(commit.clone());
        }

        // 2. Generate and commit to main trace
        tracing::info_span!("load main traces").in_scope(|| trace.load_main(pcs, main_traces));
        let (main_commit, main_data) =
            tracing::info_span!("commit to main traces").in_scope(|| trace.commit_main(pcs));
        if let Some(main_commit) = &main_commit {
            challenger.observe(main_commit.clone());
        }

        // 3. Sample permutation challenges
        let perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES] = (0..NUM_PERM_CHALLENGES)
            .map(|_| challenger.sample_ext_element::<SC::Challenge>())
            .collect_vec()
            .try_into()
            .unwrap();

        // 4. Generate and commit to permutation trace
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
        // #[cfg(feature = "debug-trace")]
        // let _ = self.write_traces_to_file::<SC>(
        //     "trace.xlsx",
        //     preprocessed_traces.iter().map(|mt| mt.matrix),
        //     main_traces.iter().map(|mt| mt.matrix),
        //     permutation_traces.iter().map(|mt| mt.matrix),
        // );
        #[cfg(debug_assertions)]
        trace.check_constraints(perm_challenges, &[]);

        // 5. Generate and commit to quotient traces
        tracing::info_span!("generate quotient trace").in_scope(|| {
            trace.generate_quotient(
                pcs,
                &pk.preprocessed.data,
                &main_data,
                &permutation_data,
                perm_challenges,
                alpha,
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

        // 6. Sample OOD point and generate opening proof
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

    // #[instrument(skip_all)]
    // fn verify<SC>(
    //     &self,
    //     config: &SC,
    //     proof: &MachineProof<SC>,
    //     vk: &VerifyingKey<SC>,
    //     challenger: &mut SC::Challenger,
    // ) -> Result<(), VerificationError>
    // where
    //     SC: StarkGenericConfig,
    //     // Val<SC>: PrimeField32,
    // {
    //     let pcs = config.pcs();
    //     let chips = self.chips();

    //     if let Some(preprocessed) = &vk.preprocessed {
    //         challenger.observe(preprocessed.commitment.clone());
    //     }

    //     let MachineProof {
    //         commitments,
    //         opening_proof,
    //         chip_proofs,
    //     } = proof;

    //     let main_degrees = chip_proofs
    //         .iter()
    //         .map(|chip_proof| chip_proof.map(|proof| proof.degree))
    //         .collect_vec();
    //     let quotient_degrees = chips
    //         .iter()
    //         .map(|&chip| get_quotient_degree::<Val<SC>, _>(chip, 0))
    //         .collect_vec();

    //     let main_domains = main_degrees
    //         .iter()
    //         .map(|&degree| degree.map(|degree| pcs.natural_domain_for_degree(degree)))
    //         .collect_vec();
    //     let quotient_domains = main_domains
    //         .iter()
    //         .zip(quotient_degrees.iter())
    //         .map(|(domain, quotient_degree)| {
    //             domain.create_disjoint_domain(domain.size() * quotient_degree)
    //         })
    //         .collect_vec();
    //     let quotient_chunks_domains = quotient_domains
    //         .into_iter()
    //         .zip(quotient_degrees.clone())
    //         .map(|(quotient_domain, quotient_degree)| {
    //             quotient_domain.split_domains(quotient_degree)
    //         })
    //         .collect_vec();

    //     // TODO: Add preprocessed and permutation size check
    //     let main_widths = chips.iter().map(|chip| chip.width()).collect_vec();
    //     for ((chip_proof, &air_width), &quotient_degree) in chip_proofs
    //         .iter()
    //         .zip(main_widths.iter())
    //         .zip(quotient_degrees.iter())
    //     {
    //         if let Some(proof) = chip_proof {
    //             if let Some(main) = &proof.opened_values.main {
    //                 if main.local.len() != air_width {
    //                     return Err(VerificationError::InvalidProofShape);
    //                 }
    //             }
    //             if let Some(main) = &proof.opened_values.main {
    //                 if main.next.len() != air_width {
    //                     return Err(VerificationError::InvalidProofShape);
    //                 }
    //             }
    //             if let Some(quotient_chunks) = &proof.opened_values.quotient_chunks {
    //                 if quotient_chunks.len() != quotient_degree {
    //                     return Err(VerificationError::InvalidProofShape);
    //                 }
    //                 if !quotient_chunks
    //                     .iter()
    //                     .all(|qc| qc.len() == <SC::Challenge as AbstractExtensionField<Val<SC>>>::D)
    //                 {
    //                     return Err(VerificationError::InvalidProofShape);
    //                 }
    //             }
    //         }
    //     }

    //     if let Some(main) = &commitments.main {
    //         challenger.observe(main.clone());
    //     }
    //     let perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES] = (0..NUM_PERM_CHALLENGES)
    //         .map(|_| challenger.sample_ext_element::<SC::Challenge>())
    //         .collect_vec()
    //         .try_into()
    //         .unwrap();
    //     if let Some(permutation) = &commitments.permutation {
    //         challenger.observe(permutation.clone());
    //     }
    //     let alpha = challenger.sample_ext_element::<SC::Challenge>();
    //     if let Some(quotient_chunks) = &commitments.quotient_chunks {
    //         challenger.observe(quotient_chunks.clone());
    //     }

    //     let zeta: SC::Challenge = challenger.sample_ext_element();

    //     let mut rounds = vec![];
    //     if let Some(preprocessed) = vk.preprocessed {
    //         let preprocessed_domains_and_openings = chip_proofs
    //             .iter()
    //             .zip_eq(preprocessed.degrees.iter())
    //             .map(|(chip_proof, _)| {
    //                 chip_proof
    //                     .as_ref()
    //                     .map(|proof| proof.opened_values.preprocessed)
    //             })
    //             .flatten()
    //             .map(|(opening, &domain)| {
    //                 (
    //                     domain,
    //                     vec![
    //                         (zeta, opening.local.clone()),
    //                         (domain.next_point(zeta).unwrap(), opening.next.clone()),
    //                     ],
    //                 )
    //             })
    //             .collect_vec();
    //         rounds.push((preprocessed.commitment, preprocessed_domains_and_openings));
    //     }
    //     let main_domains_and_openings = main_domains
    //         .iter()
    //         .zip(chip_proofs.iter())
    //         .map(|(&domain, proof)| {
    //             (
    //                 domain,
    //                 vec![
    //                     (zeta, proof.opened_values.main.local.clone()),
    //                     (
    //                         domain.next_point(zeta).unwrap(),
    //                         proof.opened_values.main.next.clone(),
    //                     ),
    //                 ],
    //             )
    //         })
    //         .collect_vec();
    //     rounds.push((commitments.main.clone(), main_domains_and_openings));

    //     let perm_domains_and_openings = chip_proofs
    //         .iter()
    //         .zip(main_domains.iter())
    //         .flat_map(|(proof, &domain)| {
    //             proof.opened_values.permutation.as_ref().map(|opening| {
    //                 (
    //                     domain,
    //                     vec![
    //                         (zeta, opening.local.clone()),
    //                         (domain.next_point(zeta).unwrap(), opening.next.clone()),
    //                     ],
    //                 )
    //             })
    //         })
    //         .collect_vec();
    //     rounds.push((commitments.permutation.clone(), perm_domains_and_openings));

    //     let quotient_chunks_domains_and_openings = quotient_chunks_domains
    //         .iter()
    //         .flatten()
    //         .zip(
    //             chip_proofs
    //                 .iter()
    //                 .flat_map(|proof| &proof.opened_values.quotient_chunks),
    //         )
    //         .map(|(&domain, opened_values)| (domain, vec![(zeta, opened_values.clone())]))
    //         .collect_vec();
    //     rounds.push((
    //         commitments.quotient_chunks.clone(),
    //         quotient_chunks_domains_and_openings,
    //     ));

    //     pcs.verify(rounds, opening_proof, challenger)
    //         .map_err(|_| VerificationError::InvalidOpeningArgument)?;

    //     for (qc_domains, chip_proof, &main_domain, &chip) in izip!(
    //         quotient_chunks_domains.iter(),
    //         chip_proofs.iter(),
    //         main_domains.iter(),
    //         chips.iter()
    //     ) {
    //         verify_constraints::<SC, _>(
    //             chip,
    //             &chip_proof.opened_values,
    //             main_domain,
    //             qc_domains,
    //             zeta,
    //             alpha,
    //             perm_challenges.as_slice(),
    //             chip_proof.cumulative_sum,
    //         )?;
    //     }

    //     let sum: SC::Challenge = proof
    //         .chip_proofs
    //         .iter()
    //         .flat_map(|chip_proof| chip_proof.cumulative_sum)
    //         .sum();
    //     if sum != SC::Challenge::zero() {
    //         return Err(VerificationError::NonZeroCumulativeSum);
    //     }

    //     Ok(())
    // }

    // #[cfg(feature = "debug-trace")]
    // fn write_traces_to_file<SC: StarkGenericConfig>(
    //     &self,
    //     path: &str,
    //     preprocessed_traces: &[Option<RowMajorMatrix<Val<SC>>>],
    //     main_traces: &[RowMajorMatrix<Val<SC>>],
    //     perm_traces: &[Option<RowMajorMatrix<SC::Challenge>>],
    // ) -> Result<(), Box<dyn Error>>
    // where
    //     Val<SC>: PrimeField32,
    // {
    //     use rust_xlsxwriter::Workbook;

    //     let chips = self.chips();
    //     let mut workbook = Workbook::new();
    //     for (chip, preprocessed_trace, main_trace, perm_trace) in
    //         izip!(chips, preprocessed_traces, main_traces, perm_traces)
    //     {
    //         let worksheet = workbook.add_worksheet();
    //         worksheet.set_name(format!("{}", chip))?;
    //         chip.write_traces_to_worksheet(worksheet, preprocessed_trace, main_trace, perm_trace)?;
    //     }

    //     workbook.save(path)?;

    //     Ok(())
    // }
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
    fn test_machine_prove() {
        // fn test_machine_prove() -> Result<(), VerificationError> {
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
        let proof = machine.prove(&config, &pk, traces, &mut challenger);

        let mut challenger = default_challenger();
        // machine.verify(&config, &proof, &mut challenger)
    }
}

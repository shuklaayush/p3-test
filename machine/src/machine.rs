use itertools::{izip, Itertools};
use p3_air::{AirBuilder, BaseAir};
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{AbstractExtensionField, AbstractField, PrimeField32};
use p3_interaction::NUM_PERM_CHALLENGES;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_maybe_rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator};
use p3_stark::{
    symbolic::get_log_quotient_degree, AdjacentOpenedValues, ChipProof, Commitments, OpenedValues,
};
use p3_uni_stark::{PackedChallenge, StarkGenericConfig, Val};
use tracing::instrument;

#[cfg(feature = "debug-trace")]
use std::error::Error;

use crate::{
    chip::ChipType,
    error::VerificationError,
    pcs::Commiter,
    proof::{MachineProof, ProverData, ProvingKey, VerifierData, VerifyingKey},
    quotient::quotient_values,
    verify::verify_constraints,
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
    fn setup<SC: StarkGenericConfig>(
        &self,
        config: &SC,
        challenger: &mut SC::Challenger,
    ) -> (ProvingKey<SC>, VerifyingKey<SC>) {
        let pcs = config.pcs();
        let chips = self.chips();

        let traces = chips.iter().map(|chip| chip.preprocessed_trace());
        let traces = pcs.load_traces(traces);

        let (pdata, vdata) = if let Some((data, commit)) = pcs.commit_traces(traces.as_slice()) {
            let packed_degrees = traces.iter().flat_map(|trace| trace.degree);
            let vdata = VerifierData {
                commitment: commit,
                degrees: packed_degrees,
            };
            let pdata = ProverData {
                data,
                commitment: commit,
            };
            (Some(pdata), Some(vdata))
        } else {
            (None, None)
        };

        let vk = VerifyingKey {
            preprocessed: vdata,
        };
        let pk = ProvingKey {
            preprocessed: pdata,
            traces,
        };

        (pk, vk)
    }

    // TODO: Move main trace generation outside
    #[instrument(skip_all)]
    fn prove<SC: StarkGenericConfig>(
        &self,
        config: &SC,
        pk: &ProvingKey<SC>,
        main_traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
        challenger: &mut SC::Challenger,
    ) -> MachineProof<SC>
    where
        Val<SC>: PrimeField32,
        <<SC as StarkGenericConfig>::Pcs as Pcs<
            <SC as StarkGenericConfig>::Challenge,
            <SC as StarkGenericConfig>::Challenger,
        >>::Domain: Send,
    {
        let chips = self.chips();
        assert_eq!(main_traces.len(), chips.len(), "Length mismatch");

        let pcs = config.pcs();

        // 1. Observe preprocessed commitment
        if let Some(preprocessed) = pk.preprocessed {
            challenger.observe(preprocessed.commitment);
        }
        let preprocessed_traces =
            tracing::info_span!("load preprocessed traces").in_scope(|| pcs.load_traces(pk.traces));

        // 2. Generate and commit to main trace
        let main_traces =
            tracing::info_span!("load main traces").in_scope(|| pcs.load_traces(main_traces));
        let (main_commit, main_data) = tracing::info_span!("commit to main traces")
            .in_scope(|| pcs.commit_traces(main_traces.as_slice()));
        if let Some(main_commit) = main_commit {
            challenger.observe(main_commit.clone());
        }

        // 3. Generate and commit to permutation trace
        let mut perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES] =
            (0..NUM_PERM_CHALLENGES).map(|_| challenger.sample_ext_element::<SC::Challenge>());
        let packed_perm_challenges = perm_challenges
            .iter()
            .map(|c| PackedChallenge::<SC>::from_f(*c))
            .collect_vec();

        let permutation_traces =
            tracing::info_span!("generate permutation traces").in_scope(|| {
                chips
                    .into_par_iter()
                    .zip_eq(preprocessed_traces.iter())
                    .zip_eq(main_traces.iter())
                    .map(|(chip, (preprocessed_trace, main_trace))| {
                        chip.generate_permutation_trace::<SC, _>(
                            &preprocessed_trace.trace,
                            &main_trace.trace,
                            chip.all_interactions(),
                            perm_challenges,
                        )
                    })
                    .collect_vec()
            });
        let permutation_traces = tracing::info_span!("load permutation traces")
            .in_scope(|| pcs.load_traces(permutation_traces));

        let (permutation_commit, permutation_data) =
            tracing::info_span!("commit to permutation traces")
                .in_scope(|| pcs.commit_traces(permutation_traces.as_slice()));
        if let Some(perm_commit) = permutation_commit {
            challenger.observe(permutation_commit.clone());
        }
        let alpha: SC::Challenge = challenger.sample_ext_element();

        let cumulative_sums = permutation_traces
            .iter()
            .map(|mt| {
                mt.as_ref().map(|trace| {
                    let matrix = trace;
                    *matrix.row_slice(matrix.height() - 1).last().unwrap()
                })
            })
            .collect_vec();

        // 4. Verify constraints
        // #[cfg(feature = "debug-trace")]
        // let _ = self.write_traces_to_file::<SC>(
        //     "trace.xlsx",
        //     preprocessed_traces.iter().map(|mt| mt.matrix),
        //     main_traces.iter().map(|mt| mt.matrix),
        //     permutation_traces.iter().map(|mt| mt.matrix),
        // );
        // #[cfg(debug_assertions)]
        // for (chip, main_trace, permutation_trace, &cumulative_sum) in izip!(
        //     chips,
        //     main_traces.iter(),
        //     permutation_traces.iter(),
        //     cumulative_sums.iter()
        // ) {
        //     check_constraints::<_, SC>(
        //         chip,
        //         main_trace,
        //         permutation_trace,
        //         &perm_challenges,
        //         cumulative_sum,
        //         &[],
        //     );
        // }
        // #[cfg(debug_assertions)]
        // check_cumulative_sums(&permutation_traces[..]);

        // 5. Generate and commit to quotient traces
        let quotient_degrees = chips
            .iter()
            .map(|&chip| {
                let d = get_log_quotient_degree::<Val<SC>, _>(chip, 0);
                1 << d
            })
            .collect_vec();
        let quotient_domains = main_traces
            .iter()
            .zip_eq(quotient_degrees.iter())
            .map(|(trace, quotient_degree)| {
                let domain = trace.domain;
                domain.create_disjoint_domain(domain.size() * quotient_degree)
            })
            .collect_vec();

        let quotient_values = quotient_domains
            .clone()
            .into_par_iter()
            .zip_eq(preprocessed_traces.par_iter())
            .zip_eq(main_traces.par_iter())
            .zip_eq(permutation_traces.par_iter())
            .zip_eq(chips.par_iter())
            .enumerate()
            .map(
                |(
                    i,
                    (
                        quotient_domain,
                        (preprocessed_trace, (main_trace, (permutation_trace, &chip))),
                    ),
                )| {
                    let preprocessed_trace_on_quotient_domains =
                        if let Some(preprocessed_data) = pk.preprocessed {
                            let index = preprocessed_trace
                                .expect("Index shouldn't be None")
                                .opening_index;
                            pcs.get_evaluations_on_domain(preprocessed_data, index, quotient_domain)
                                .to_row_major_matrix()
                        } else {
                            RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                        };
                    let main_trace_on_quotient_domains = pcs
                        .get_evaluations_on_domain(&main_data, i, quotient_domain)
                        .to_row_major_matrix();
                    let perm_trace_on_quotient_domains =
                        if let Some(permutation_data) = permutation_data {
                            let index = permutation_trace
                                .expect("Index shouldn't be None")
                                .opening_index;
                            pcs.get_evaluations_on_domain(permutation_data, index, quotient_domain)
                                .to_row_major_matrix()
                        } else {
                            RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                        };
                    quotient_values::<SC, _, _>(
                        chip,
                        main_traces.domain,
                        quotient_domain,
                        preprocessed_trace_on_quotient_domains,
                        main_trace_on_quotient_domains,
                        perm_trace_on_quotient_domains,
                        &packed_perm_challenges,
                        alpha,
                        cumulative_sums[i],
                    )
                },
            )
            .collect_vec();
        let quotient_chunks = quotient_domains
            .clone()
            .into_iter()
            .zip_eq(quotient_degrees.clone())
            .zip_eq(quotient_values)
            .map(|((domain, degree), values)| {
                let quotient_flat = RowMajorMatrix::new_col(values).flatten_to_base();
                domain.split_evals(degree, quotient_flat)
            })
            .collect_vec();
        let qc_domains = quotient_domains
            .into_iter()
            .zip_eq(quotient_degrees.clone())
            .map(|(quotient_domain, quotient_degree)| {
                quotient_domain.split_domains(quotient_degree)
            })
            .collect_vec();

        let (quotient_commit, quotient_data) = tracing::info_span!("commit to quotient chunks")
            .in_scope(|| {
                pcs.commit(
                    qc_domains
                        .into_iter()
                        .flatten()
                        .zip_eq(quotient_chunks.into_iter().flatten())
                        .collect_vec(),
                )
            });
        challenger.observe(quotient_commit.clone());

        let commitments = Commitments {
            main: main_commit,
            permutation: permutation_commit,
            quotient_chunks: quotient_commit,
        };

        let zeta: SC::Challenge = challenger.sample_ext_element();

        let mut rounds = vec![];
        if let Some(preprocessed_data) = pk.preprocessed {
            let preprocessed_domains = pk
                .degrees
                .flat_map(|md| md.map(|degree| pcs.natural_domain_for_degree(degree)));
            let opening_points = preprocessed_domains
                .iter()
                .map(|domain| vec![zeta, domain.next_point(zeta).unwrap()])
                .collect_vec();
            rounds.push((preprocessed_data, opening_points));
        }

        let main_opening_points = main_traces
            .iter()
            .flat_map(|trace| {
                let domain = trace.domain;
                vec![zeta, domain.next_point(zeta).unwrap()]
            })
            .collect_vec();
        rounds.push((&main_data, main_opening_points));

        if let Some(permutation_data) = permutation_data {
            let perm_opening_points = permutation_traces
                .iter()
                .flat_map(|trace| {
                    let domain = trace.domain;
                    vec![zeta, domain.next_point(zeta).unwrap()]
                })
                .collect_vec();
            rounds.push((&permutation_data, perm_opening_points));
        }

        // open every chunk at zeta
        let quotient_opening_points = quotient_degrees
            .iter()
            .flat_map(|&quotient_degree| (0..quotient_degree).map(|_| vec![zeta]).collect_vec())
            .collect_vec();
        rounds.push((&quotient_data, quotient_opening_points));

        let (opening_values, opening_proof) = pcs.open(rounds);

        let quotient_openings = opening_values.pop();
        // Unflatten quotient openings
        let quotient_openings = quotient_degrees
            .iter()
            .scan(0, |start, &chunk_size| {
                let end = *start + chunk_size;
                let res = Some(quotient_openings[*start..end].to_vec());
                *start = end;
                res
            })
            .collect_vec();

        let permutation_openings = if let Some(_) = permutation_data {
            opening_values.pop()
        } else {
            None
        };
        let permutation_openings = permutation_traces
            .iter()
            .map(|mt| mt.map(|trace| permutation_openings[trace.opening_index].clone()))
            .collect_vec();

        let main_openings = opening_values.pop();

        let preprocessed_openings = if let Some(_) = pk.preprocessed {
            opening_values.pop()
        } else {
            None
        };
        let preprocessed_openings = preprocessed_traces
            .iter()
            .map(|mt| mt.map(|trace| preprocessed_openings[trace.opening_index].clone()))
            .collect_vec();

        let chip_proofs = izip!(
            main_traces,
            preprocessed_openings,
            main_openings,
            permutation_openings,
            quotient_openings,
            cumulative_sums,
        )
        .map(
            |(main_trace, preprocessed, main, perm, quotient, cumulative_sum)| {
                let preprocessed = preprocessed.map(|openings| {
                    assert_eq!(openings.len(), 2, "Should have 2 openings");
                    AdjacentOpenedValues {
                        local: openings[0].clone(),
                        next: openings[1].clone(),
                    }
                });
                let [main_local, main_next] = main.try_into().expect("Should have 2 openings");
                let perm = perm.map(|openings| {
                    assert_eq!(openings.len(), 2, "Should have 2 openings");
                    AdjacentOpenedValues {
                        local: openings[0].clone(),
                        next: openings[1].clone(),
                    }
                });
                let quotient_chunks = quotient
                    .into_iter()
                    .map(|mut chunk| chunk.remove(0))
                    .collect_vec();
                let opened_values = OpenedValues {
                    preprocessed,
                    main: AdjacentOpenedValues {
                        local: main_local,
                        next: main_next,
                    },
                    permutation: perm,
                    quotient_chunks,
                };
                ChipProof {
                    degree: main_trace.degree,
                    opened_values,
                    cumulative_sum,
                }
            },
        )
        .collect_vec();

        MachineProof {
            commitments,
            opening_proof,
            chip_proofs,
        }
    }

    #[instrument(skip_all)]
    fn verify<SC: StarkGenericConfig>(
        &self,
        config: &SC,
        proof: &MachineProof<SC>,
        vk: &VerifyingKey<SC>,
        challenger: &mut SC::Challenger,
    ) -> Result<(), VerificationError>
    where
        Val<SC>: PrimeField32,
    {
        let pcs = config.pcs();
        let chips = self.chips();

        if let Some(preprocessed) = vk.preprocessed {
            challenger.observe(preprocessed.commitment.clone());
        }

        let MachineProof {
            commitments,
            opening_proof,
            chip_proofs,
        } = proof;

        let main_degrees = chip_proofs
            .iter()
            .map(|chip_proof| chip_proof.degree)
            .collect_vec();
        let quotient_degrees = chips
            .iter()
            .map(|&chip| {
                let d = get_log_quotient_degree::<Val<SC>, _>(chip, 0);
                1 << d
            })
            .collect_vec();

        let main_domains = main_degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect_vec();
        let quotient_domains = main_domains
            .iter()
            .zip(quotient_degrees.iter())
            .map(|(domain, quotient_degree)| {
                domain.create_disjoint_domain(domain.size() * quotient_degree)
            })
            .collect_vec();
        let quotient_chunks_domains = quotient_domains
            .into_iter()
            .zip(quotient_degrees.clone())
            .map(|(quotient_domain, quotient_degree)| {
                quotient_domain.split_domains(quotient_degree)
            })
            .collect_vec();

        let main_widths = chips.iter().map(|chip| chip.width()).collect_vec();
        // TODO: Add preprocessed and permutation size check
        let valid_shape =
            chip_proofs
                .iter()
                .zip(main_widths.iter())
                .zip(quotient_degrees.iter())
                .all(|((chip_proof, &air_width), &quotient_degree)| {
                    chip_proof.opened_values.main.local.len() == air_width
                        && chip_proof.opened_values.main.next.len() == air_width
                        && chip_proof.opened_values.quotient_chunks.len() == quotient_degree
                        && chip_proof.opened_values.quotient_chunks.iter().all(|qc| {
                            qc.len() == <SC::Challenge as AbstractExtensionField<Val<SC>>>::D
                        })
                });
        if !valid_shape {
            return Err(VerificationError::InvalidProofShape);
        }

        challenger.observe(commitments.main_trace.clone());
        let mut perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES] =
            (0..NUM_PERM_CHALLENGES).map(|_| challenger.sample_ext_element::<SC::Challenge>());
        challenger.observe(commitments.perm_trace.clone());
        let alpha = challenger.sample_ext_element::<SC::Challenge>();
        challenger.observe(commitments.quotient_chunks.clone());

        let zeta: SC::Challenge = challenger.sample_ext_element();

        let mut rounds = vec![];
        if let Some(preprocessed) = vk.preprocessed {
            let preprocessed_domains_and_openings = chip_proofs
                .iter()
                .flat_map(|proof| proof.opened_values.preprocessed.as_ref())
                .zip_eq(preprocessed.degrees.iter())
                .map(|(opening, &domain)| {
                    (
                        domain,
                        vec![
                            (zeta, opening.local.clone()),
                            (domain.next_point(zeta).unwrap(), opening.next.clone()),
                        ],
                    )
                })
                .collect_vec();
            rounds.push((preprocessed.commitment, preprocessed_domains_and_openings));
        }
        let main_domains_and_openings = main_domains
            .iter()
            .zip(chip_proofs.iter())
            .map(|(&domain, proof)| {
                (
                    domain,
                    vec![
                        (zeta, proof.opened_values.main.local.clone()),
                        (
                            domain.next_point(zeta).unwrap(),
                            proof.opened_values.main.next.clone(),
                        ),
                    ],
                )
            })
            .collect_vec();
        rounds.push((commitments.main.clone(), main_domains_and_openings));

        let perm_domains_and_openings = chip_proofs
            .iter()
            .zip(main_domains.iter())
            .flat_map(|(proof, &domain)| {
                proof.opened_values.permutation.as_ref().map(|opening| {
                    (
                        domain,
                        vec![
                            (zeta, opening.local.clone()),
                            (domain.next_point(zeta).unwrap(), opening.next.clone()),
                        ],
                    )
                })
            })
            .collect_vec();
        rounds.push((commitments.perm_trace.clone(), perm_domains_and_openings));

        let quotient_chunks_domains_and_openings = quotient_chunks_domains
            .iter()
            .flatten()
            .zip(
                chip_proofs
                    .iter()
                    .flat_map(|proof| &proof.opened_values.quotient_chunks),
            )
            .map(|(&domain, opened_values)| (domain, vec![(zeta, opened_values.clone())]))
            .collect_vec();
        rounds.push((
            commitments.quotient_chunks.clone(),
            quotient_chunks_domains_and_openings,
        ));

        pcs.verify(rounds, opening_proof, challenger)
            .map_err(|_| VerificationError::InvalidOpeningArgument)?;

        for (qc_domains, chip_proof, &main_domain, &chip) in izip!(
            quotient_chunks_domains.iter(),
            chip_proofs.iter(),
            main_domains.iter(),
            chips.iter()
        ) {
            verify_constraints::<SC, _>(
                chip,
                &chip_proof.opened_values,
                main_domain,
                qc_domains,
                zeta,
                alpha,
                perm_challenges.as_slice(),
                chip_proof.cumulative_sum,
            )?;
        }

        let sum: SC::Challenge = proof
            .chip_proofs
            .iter()
            .flat_map(|chip_proof| chip_proof.cumulative_sum)
            .sum();
        if sum != SC::Challenge::zero() {
            return Err(VerificationError::NonZeroCumulativeSum);
        }

        Ok(())
    }

    #[cfg(feature = "debug-trace")]
    fn write_traces_to_file<SC: StarkGenericConfig>(
        &self,
        path: &str,
        preprocessed_traces: &[Option<RowMajorMatrix<Val<SC>>>],
        main_traces: &[RowMajorMatrix<Val<SC>>],
        perm_traces: &[Option<RowMajorMatrix<SC::Challenge>>],
    ) -> Result<(), Box<dyn Error>>
    where
        Val<SC>: PrimeField32,
    {
        use rust_xlsxwriter::Workbook;

        let chips = self.chips();
        let mut workbook = Workbook::new();
        for (chip, preprocessed_trace, main_trace, perm_trace) in
            izip!(chips, preprocessed_traces, main_traces, perm_traces)
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(format!("{}", chip))?;
            chip.write_traces_to_worksheet(worksheet, preprocessed_trace, main_trace, perm_trace)?;
        }

        workbook.save(path)?;

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::config::{default_challenger, default_config};

//     use p3_keccak::KeccakF;
//     use p3_symmetric::{PseudoCompressionFunction, TruncatedPermutation};
//     use rand::{random, thread_rng, Rng};
//     use tracing_forest::{util::LevelFilter, ForestLayer};
//     use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

//     fn generate_digests(leaf_hashes: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
//         let keccak = TruncatedPermutation::new(KeccakF {});
//         let mut digests = vec![leaf_hashes.to_vec()];

//         while let Some(last_level) = digests.last().cloned() {
//             if last_level.len() == 1 {
//                 break;
//             }

//             let next_level = last_level
//                 .chunks_exact(2)
//                 .map(|chunk| keccak.compress([chunk[0], chunk[1]]))
//                 .collect();

//             digests.push(next_level);
//         }

//         digests
//     }

//     #[test]
//     fn test_machine_prove() -> Result<(), VerificationError> {
//         let env_filter = EnvFilter::builder()
//             .with_default_directive(LevelFilter::INFO.into())
//             .from_env_lossy();

//         Registry::default()
//             .with(env_filter)
//             .with(ForestLayer::default())
//             .init();

//         const NUM_BYTES: usize = 1000;
//         let preimage = (0..NUM_BYTES).map(|_| random()).collect_vec();

//         const HEIGHT: usize = 8;
//         let leaf_hashes = (0..2u64.pow(HEIGHT as u32)).map(|_| random()).collect_vec();
//         let digests = generate_digests(&leaf_hashes);

//         let leaf_index = thread_rng().gen_range(0..leaf_hashes.len());
//         let machine = Machine::new(preimage, digests, leaf_index);

//         let config = default_config();
//         let mut challenger = default_challenger();
//         let proof = machine.prove(&config, &mut challenger);

//         let mut challenger = default_challenger();
//         machine.verify(&config, &proof, &mut challenger)
//     }
// }

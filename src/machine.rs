use p3_air::{Air, AirBuilder, BaseAir, TwoRowMatrixView};
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{AbstractExtensionField, AbstractField, Field, PrimeField64};
use p3_matrix::{dense::RowMajorMatrix, Matrix, MatrixRowSlices};
use p3_maybe_rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator};
use p3_uni_stark::{get_log_quotient_degree, StarkGenericConfig, Val};
use p3_util::log2_strict_usize;

use crate::{
    chip::{
        check_constraints, check_cumulative_sums, eval_permutation_constraints,
        generate_permutation_trace, Chip, Interaction, InteractionType, MachineChip,
    },
    error::VerificationError,
    folder::VerifierConstraintFolder,
    keccak_permute::KeccakPermuteChip,
    merkle_tree::MerkleTreeChip,
    proof::{ChipProof, Commitments, MachineProof, OpenedValues},
    quotient::quotient_values,
};

pub enum ChipType {
    KeccakPermute(KeccakPermuteChip),
    MerkleTree(MerkleTreeChip),
}

impl<F: Field> BaseAir<F> for ChipType {
    fn width(&self) -> usize {
        match self {
            ChipType::KeccakPermute(chip) => <KeccakPermuteChip as BaseAir<F>>::width(chip),
            ChipType::MerkleTree(chip) => <MerkleTreeChip as BaseAir<F>>::width(chip),
        }
    }

    fn preprocessed_trace(&self) -> Option<RowMajorMatrix<F>> {
        match self {
            ChipType::KeccakPermute(chip) => chip.preprocessed_trace(),
            ChipType::MerkleTree(chip) => chip.preprocessed_trace(),
        }
    }
}

impl<AB: AirBuilder> Air<AB> for ChipType {
    fn eval(&self, builder: &mut AB) {
        match self {
            ChipType::KeccakPermute(chip) => chip.eval(builder),
            ChipType::MerkleTree(chip) => chip.eval(builder),
        }
    }
}

impl<F: PrimeField64> Chip<F> for ChipType {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        match self {
            ChipType::KeccakPermute(chip) => chip.generate_trace(),
            ChipType::MerkleTree(chip) => chip.generate_trace(),
        }
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        match self {
            ChipType::KeccakPermute(chip) => chip.sends(),
            ChipType::MerkleTree(chip) => chip.sends(),
        }
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        match self {
            ChipType::KeccakPermute(chip) => chip.receives(),
            ChipType::MerkleTree(chip) => chip.receives(),
        }
    }

    fn all_interactions(&self) -> Vec<(Interaction<F>, InteractionType)> {
        match self {
            ChipType::KeccakPermute(chip) => chip.all_interactions(),
            ChipType::MerkleTree(chip) => chip.all_interactions(),
        }
    }
}

impl<SC: StarkGenericConfig> MachineChip<SC> for ChipType
where
    Val<SC>: PrimeField64,
{
    fn trace_width(&self) -> usize {
        match self {
            ChipType::KeccakPermute(chip) => {
                <KeccakPermuteChip as MachineChip<SC>>::trace_width(chip)
            }
            ChipType::MerkleTree(chip) => <MerkleTreeChip as MachineChip<SC>>::trace_width(chip),
        }
    }
}

pub struct Machine {
    keccak_permute_chip: ChipType,
    merkle_tree_chip: ChipType,
}

impl Machine {
    pub fn chips(&self) -> Vec<&ChipType> {
        vec![&self.keccak_permute_chip, &self.merkle_tree_chip]
    }
}

impl Machine {
    pub fn new(digests: Vec<Vec<[u8; 32]>>, leaf_index: usize) -> Self {
        let leaf = digests[0][leaf_index];

        let height = digests.len() - 1;
        let siblings = (0..height)
            .map(|i| digests[i][(leaf_index >> i) ^ 1])
            .collect::<Vec<[u8; 32]>>();
        let keccak_inputs = (0..height)
            .map(|i| {
                let index = leaf_index >> i;
                let parity = index & 1;
                let (left, right) = if parity == 0 {
                    (digests[i][index], digests[i][index ^ 1])
                } else {
                    (digests[i][index ^ 1], digests[i][index])
                };
                let mut input = [0; 25];
                input[0..4].copy_from_slice(
                    left.chunks_exact(8)
                        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                        .collect::<Vec<_>>()
                        .as_slice(),
                );
                input[4..8].copy_from_slice(
                    right
                        .chunks_exact(8)
                        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                        .collect::<Vec<_>>()
                        .as_slice(),
                );
                input
            })
            .collect::<Vec<_>>();

        let keccak_permute_chip = KeccakPermuteChip {
            inputs: keccak_inputs,
        };

        let merkle_tree_chip = MerkleTreeChip {
            leaves: vec![leaf],
            leaf_indices: vec![leaf_index],
            siblings: vec![siblings.try_into().unwrap()],
        };

        Self {
            keccak_permute_chip: ChipType::KeccakPermute(keccak_permute_chip),
            merkle_tree_chip: ChipType::MerkleTree(merkle_tree_chip),
        }
    }

    fn prove<SC: StarkGenericConfig>(
        &self,
        config: &SC,
        challenger: &mut SC::Challenger,
    ) -> MachineProof<SC>
    where
        Val<SC>: PrimeField64,
        <<SC as StarkGenericConfig>::Pcs as Pcs<
            <SC as StarkGenericConfig>::Challenge,
            <SC as StarkGenericConfig>::Challenger,
        >>::Domain: Send,
    {
        let pcs = config.pcs();

        // 1. Generate and commit to preprocessed traces
        // let preprocessed_traces: Vec<RowMajorMatrix<Val<SC>>> =
        //     tracing::info_span!("generate preprocessed traces").in_scope(|| {
        //         self.chips()
        //             .par_iter()
        //             .flat_map(|chip| chip.preprocessed_trace())
        //             .collect::<Vec<_>>()
        //     });
        // let preprocessed_degrees: [usize; 2usize] = preprocessed_traces
        //     .iter()
        //     .map(|trace| trace.height())
        //     .collect::<Vec<_>>()
        //     .try_into()
        //     .unwrap();
        // let preprocessed_domains =
        //     preprocessed_degrees.map(|degree| pcs.natural_domain_for_degree(degree));
        // let (preprocessed_commit, preprocessed_data) =
        //     tracing::info_span!("commit to preprocessed traces").in_scope(|| {
        //         pcs.commit(
        //             std::iter::zip(preprocessed_domains, preprocessed_traces).collect::<Vec<_>>(),
        //         )
        //     });
        // challenger.observe(preprocessed_commit.clone());
        // let mut preprocessed_trace_ldes = pcs.get_ldes(&preprocessed_data);

        // 2. Generate and commit to main trace
        let main_traces = tracing::info_span!("generate main traces").in_scope(|| {
            self.chips()
                .par_iter()
                .map(|chip| chip.generate_trace())
                .collect::<Vec<_>>()
        });
        let main_degrees = main_traces
            .iter()
            .map(|trace| trace.height())
            .collect::<Vec<_>>();
        let main_domains = main_degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect::<Vec<_>>();

        let (main_commit, main_data) =
            tracing::info_span!("commit to main traces").in_scope(|| {
                pcs.commit(
                    std::iter::zip(main_domains.clone(), main_traces.clone()).collect::<Vec<_>>(),
                )
            });
        challenger.observe(main_commit.clone());

        // 3. Generate and commit to permutation trace
        let mut perm_challenges = Vec::new();
        for _ in 0..2 {
            perm_challenges.push(challenger.sample_ext_element::<SC::Challenge>());
        }

        let perm_traces = tracing::info_span!("generate permutation traces").in_scope(|| {
            self.chips()
                .into_par_iter()
                .enumerate()
                .map(|(i, chip)| {
                    generate_permutation_trace::<SC, _>(
                        chip,
                        &main_traces[i],
                        perm_challenges.clone(),
                    )
                })
                .collect::<Vec<_>>()
        });
        // TODO: Assert equal to main trace degrees?
        let perm_degrees = perm_traces
            .iter()
            .map(|trace: &RowMajorMatrix<SC::Challenge>| trace.height())
            .collect::<Vec<_>>();
        let perm_domains = perm_degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect::<Vec<_>>();
        let (perm_commit, perm_data) = tracing::info_span!("commit to permutation traces")
            .in_scope(|| {
                let flattened_perm_traces = perm_traces
                    .iter()
                    .map(|trace| trace.flatten_to_base())
                    .collect::<Vec<_>>();
                pcs.commit(std::iter::zip(perm_domains, flattened_perm_traces).collect::<Vec<_>>())
            });
        challenger.observe(perm_commit.clone());
        let alpha: SC::Challenge = challenger.sample_ext_element();
        let cumulative_sums = perm_traces
            .iter()
            .map(|trace| *trace.row_slice(trace.height() - 1).last().unwrap())
            .collect::<Vec<_>>();

        // 4. Verify constraints
        #[cfg(debug_assertions)]
        self.chips()
            .iter()
            .zip(main_traces.iter())
            .zip(perm_traces.iter())
            .for_each(|((&chip, main_trace), perm_trace)| {
                check_constraints::<_, SC>(chip, main_trace, perm_trace, &perm_challenges, &vec![]);
            });
        #[cfg(debug_assertions)]
        check_cumulative_sums(&perm_traces[..]);

        // 5. Generate and commit to quotient traces
        let log_degrees = main_degrees
            .iter()
            .map(|&d| log2_strict_usize(d))
            .collect::<Vec<_>>();
        let log_quotient_degrees = self
            .chips()
            .iter()
            .map(|&chip| get_log_quotient_degree::<Val<SC>, _>(chip, 0))
            .collect::<Vec<_>>();
        let quotient_degrees = log_quotient_degrees
            .iter()
            .map(|d| 1 << d)
            .collect::<Vec<_>>();
        let quotient_domains = main_domains
            .iter()
            .zip(log_degrees.iter())
            .zip(log_quotient_degrees.iter())
            .map(|((domain, log_degree), log_quotient_degree)| {
                domain.create_disjoint_domain(1 << (log_degree + log_quotient_degree))
            })
            .collect::<Vec<_>>();

        let quotient_values = quotient_domains
            .clone()
            .into_par_iter()
            .zip(self.chips().par_iter())
            .enumerate()
            .map(|(i, (quotient_domain, &chip))| {
                // let ppt: Option<RowMajorMatrix<Val<SC>>> =
                //     self.keccak_permute_chip.preprocessed_trace();
                // let preprocessed_trace_lde = ppt.map(|trace| preprocessed_trace_ldes.remove(0));
                let main_trace_on_quotient_domains =
                    pcs.get_evaluations_on_domain(&main_data, i, quotient_domain);
                let permutation_trace_on_quotient_domains =
                    pcs.get_evaluations_on_domain(&perm_data, i, quotient_domain);
                quotient_values::<SC, _, _>(
                    chip,
                    cumulative_sums[i],
                    main_domains[i],
                    quotient_domain,
                    main_trace_on_quotient_domains,
                    permutation_trace_on_quotient_domains,
                    &perm_challenges,
                    alpha,
                )
            })
            .collect::<Vec<_>>();
        let quotient_chunks = quotient_domains
            .clone()
            .into_iter()
            .zip(quotient_degrees.clone())
            .zip(quotient_values)
            .map(|((domain, degree), values)| {
                let quotient_flat = RowMajorMatrix::new_col(values).flatten_to_base();
                domain.split_evals(degree, quotient_flat)
            })
            .collect::<Vec<_>>();
        let qc_domains = quotient_domains
            .into_iter()
            .zip(quotient_degrees.clone())
            .map(|(quotient_domain, quotient_degree)| {
                quotient_domain.split_domains(quotient_degree)
            })
            .collect::<Vec<_>>();

        let (quotient_commit, quotient_data) = tracing::info_span!("commit to quotient chunks")
            .in_scope(|| {
                pcs.commit(
                    qc_domains
                        .into_iter()
                        .flatten()
                        .zip(quotient_chunks.into_iter().flatten())
                        .collect::<Vec<_>>(),
                )
            });
        challenger.observe(quotient_commit.clone());

        let commitments = Commitments {
            main_trace: main_commit,
            perm_trace: perm_commit,
            quotient_chunks: quotient_commit,
        };

        let zeta: SC::Challenge = challenger.sample_ext_element();
        let zeta_and_next = main_domains
            .iter()
            .map(|domain| vec![zeta, domain.next_point(zeta).unwrap()])
            .collect::<Vec<_>>();

        let (opening_values, opening_proof) = pcs.open(
            vec![
                (&main_data, zeta_and_next.clone()),
                (&perm_data, zeta_and_next),
                (
                    &quotient_data,
                    // open every chunk at zeta
                    quotient_degrees
                        .iter()
                        .flat_map(|&quotient_degree| {
                            (0..quotient_degree).map(|_| vec![zeta]).collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>(),
                ),
            ],
            challenger,
        );
        let [main_openings, perm_openings, quotient_openings] = opening_values
            .try_into()
            .expect("Should have 3 rounds of openings");
        let quotient_openings = quotient_degrees
            .iter()
            .scan(0, |start, &chunk_size| {
                let end = *start + chunk_size;
                let res = Some(quotient_openings[*start..end].to_vec());
                *start = end;
                res
            })
            .collect::<Vec<_>>();

        let chip_proofs = log_degrees
            .iter()
            .zip(main_openings)
            .zip(perm_openings)
            .zip(quotient_openings)
            .zip(perm_traces)
            .map(|((((log_degree, main), perm), quotient), perm_trace)| {
                let [main_local, main_next] = main.try_into().expect("Should have 2 openings");
                let [perm_local, perm_next] = perm.try_into().expect("Should have 2 openings");
                let quotient_chunks = quotient
                    .into_iter()
                    .map(|mut chunk| chunk.remove(0))
                    .collect::<Vec<_>>();
                let opened_values = OpenedValues {
                    trace_local: main_local,
                    trace_next: main_next,
                    permutation_local: perm_local,
                    permutation_next: perm_next,
                    quotient_chunks,
                };
                let cumulative_sum = *perm_trace
                    .row_slice(perm_trace.height() - 1)
                    .last()
                    .unwrap();
                ChipProof {
                    degree_bits: *log_degree,
                    opened_values,
                    cumulative_sum,
                }
            })
            .collect::<Vec<_>>();

        MachineProof {
            commitments,
            opening_proof,
            chip_proofs,
        }
    }

    fn verify<SC: StarkGenericConfig>(
        &self,
        config: &SC,
        proof: &MachineProof<SC>,
        challenger: &mut SC::Challenger,
    ) -> Result<(), VerificationError>
    where
        Val<SC>: PrimeField64,
    {
        let MachineProof {
            commitments,
            opening_proof,
            chip_proofs,
        } = proof;
        let log_degrees = chip_proofs
            .iter()
            .map(|chip_proof| chip_proof.degree_bits)
            .collect::<Vec<_>>();

        let degrees = log_degrees
            .iter()
            .map(|degree_bits| 1 << degree_bits)
            .collect::<Vec<_>>();
        let log_quotient_degrees = self
            .chips()
            .iter()
            .map(|&chip| get_log_quotient_degree::<Val<SC>, _>(chip, 0))
            .collect::<Vec<_>>();
        let quotient_degrees = log_quotient_degrees
            .iter()
            .map(|&log_degree| 1 << log_degree)
            .collect::<Vec<_>>();

        let pcs = config.pcs();
        let main_domains = degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect::<Vec<_>>();
        let quotient_domains = main_domains
            .iter()
            .zip(log_degrees.iter())
            .zip(log_quotient_degrees.iter())
            .map(|((domain, log_degree), log_quotient_degree)| {
                domain.create_disjoint_domain(1 << (log_degree + log_quotient_degree))
            })
            .collect::<Vec<_>>();
        let quotient_chunks_domains = quotient_domains
            .into_iter()
            .zip(quotient_degrees.clone())
            .map(|(quotient_domain, quotient_degree)| {
                quotient_domain.split_domains(quotient_degree)
            })
            .collect::<Vec<_>>();

        let air_widths = self
            .chips()
            .iter()
            .map(|chip| <ChipType as BaseAir<Val<SC>>>::width(chip))
            .collect::<Vec<_>>();

        let valid_shape =
            chip_proofs
                .iter()
                .zip(air_widths.iter())
                .zip(quotient_degrees.iter())
                .all(|((chip_proof, &air_width), &quotient_degree)| {
                    chip_proof.opened_values.trace_local.len() == air_width
                        && chip_proof.opened_values.trace_next.len() == air_width
                        && chip_proof.opened_values.quotient_chunks.len() == quotient_degree
                        && chip_proof.opened_values.quotient_chunks.iter().all(|qc| {
                            qc.len() == <SC::Challenge as AbstractExtensionField<Val<SC>>>::D
                        })
                });
        if !valid_shape {
            return Err(VerificationError::InvalidProofShape);
        }

        // let preprocessed_traces: Vec<RowMajorMatrix<Val<SC>>> =
        //     tracing::info_span!("generate preprocessed traces").in_scope(|| {
        //         self.chips()
        //             .par_iter()
        //             .flat_map(|chip| chip.preprocessed_trace())
        //             .collect::<Vec<_>>()
        //     });
        // let preprocessed_degrees = preprocessed_traces
        //     .iter()
        //     .map(|trace| trace.height())
        //     .collect::<Vec<_>>();
        // let preprocessed_domains = preprocessed_degrees
        //     .iter()
        //     .map(|&degree| pcs.natural_domain_for_degree(degree))
        //     .collect();
        // let (preprocessed_commit, preprocessed_data) =
        //     tracing::info_span!("commit to preprocessed traces").in_scope(|| {
        //         pcs.commit(
        //             std::iter::zip(preprocessed_domains, preprocessed_traces).collect::<Vec<_>>(),
        //         )
        //     });
        // challenger.observe(preprocessed_commit.clone());

        challenger.observe(commitments.main_trace.clone());
        let mut perm_challenges = Vec::new();
        for _ in 0..2 {
            perm_challenges.push(challenger.sample_ext_element::<SC::Challenge>());
        }
        challenger.observe(commitments.perm_trace.clone());
        let alpha = challenger.sample_ext_element::<SC::Challenge>();
        challenger.observe(commitments.quotient_chunks.clone());

        let zeta: SC::Challenge = challenger.sample_ext_element();
        let zeta_nexts = main_domains
            .iter()
            .map(|domain| domain.next_point(zeta).unwrap())
            .collect::<Vec<_>>();

        pcs.verify(
            vec![
                (
                    commitments.main_trace.clone(),
                    main_domains
                        .iter()
                        .zip(chip_proofs.iter())
                        .zip(zeta_nexts.iter())
                        .map(|((&domain, proof), &zeta_next)| {
                            (
                                domain,
                                vec![
                                    (zeta, proof.opened_values.trace_local.clone()),
                                    (zeta_next, proof.opened_values.trace_next.clone()),
                                ],
                            )
                        })
                        .collect::<Vec<_>>(),
                ),
                (
                    commitments.perm_trace.clone(),
                    main_domains
                        .iter()
                        .zip(chip_proofs.iter())
                        .zip(zeta_nexts.iter())
                        .map(|((&domain, proof), &zeta_next)| {
                            (
                                domain,
                                vec![
                                    (zeta, proof.opened_values.permutation_local.clone()),
                                    (zeta_next, proof.opened_values.permutation_next.clone()),
                                ],
                            )
                        })
                        .collect::<Vec<_>>(),
                ),
                (
                    commitments.quotient_chunks.clone(),
                    quotient_chunks_domains
                        .iter()
                        .flatten()
                        .zip(
                            chip_proofs
                                .iter()
                                .flat_map(|proof| &proof.opened_values.quotient_chunks),
                        )
                        .map(|(&domain, opened_values)| {
                            (domain, vec![(zeta, opened_values.clone())])
                        })
                        .collect::<Vec<_>>(),
                ),
            ],
            opening_proof,
            challenger,
        )
        .map_err(|_| VerificationError::InvalidOpeningArgument)?;

        for (((qc_domains, chip_proof), main_domain), &chip) in quotient_chunks_domains
            .iter()
            .zip(chip_proofs.iter())
            .zip(main_domains.iter())
            .zip(self.chips().iter())
        {
            let zps = qc_domains
                .iter()
                .enumerate()
                .map(|(i, domain)| {
                    qc_domains
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, other_domain)| {
                            other_domain.zp_at_point(zeta)
                                * other_domain.zp_at_point(domain.first_point()).inverse()
                        })
                        .product::<SC::Challenge>()
                })
                .collect::<Vec<_>>();

            let quotient = chip_proof
                .opened_values
                .quotient_chunks
                .iter()
                .enumerate()
                .map(|(ch_i, ch)| {
                    ch.iter()
                        .enumerate()
                        .map(|(e_i, &c)| zps[ch_i] * SC::Challenge::monomial(e_i) * c)
                        .sum::<SC::Challenge>()
                })
                .sum::<SC::Challenge>();

            let sels = main_domain.selectors_at_point(zeta);

            let unflatten = |v: &[SC::Challenge]| {
                v.chunks_exact(SC::Challenge::D)
                    .map(|chunk| {
                        chunk
                            .iter()
                            .enumerate()
                            .map(|(e_i, &c)| SC::Challenge::monomial(e_i) * c)
                            .sum()
                    })
                    .collect::<Vec<SC::Challenge>>()
            };

            let mut folder: VerifierConstraintFolder<'_, SC> = VerifierConstraintFolder {
                preprocessed: TwoRowMatrixView {
                    local: &[],
                    next: &[],
                },
                main: TwoRowMatrixView {
                    local: &chip_proof.opened_values.trace_local,
                    next: &chip_proof.opened_values.trace_next,
                },
                perm: TwoRowMatrixView {
                    local: &unflatten(&chip_proof.opened_values.permutation_local),
                    next: &unflatten(&chip_proof.opened_values.permutation_next),
                },
                perm_challenges: &perm_challenges,
                public_values: &vec![],
                is_first_row: sels.is_first_row,
                is_last_row: sels.is_last_row,
                is_transition: sels.is_transition,
                alpha,
                accumulator: SC::Challenge::zero(),
            };
            chip.eval(&mut folder);
            eval_permutation_constraints::<_, SC, _>(chip, &mut folder, chip_proof.cumulative_sum);

            let folded_constraints = folder.accumulator;
            // Finally, check that
            //     folded_constraints(zeta) / Z_H(zeta) = quotient(zeta)
            if folded_constraints * sels.inv_zeroifier != quotient {
                return Err(VerificationError::OodEvaluationMismatch);
            }
        }

        let sum: SC::Challenge = proof
            .chip_proofs
            .iter()
            .map(|chip_proof| chip_proof.cumulative_sum)
            .sum();
        if sum != SC::Challenge::zero() {
            return Err(VerificationError::NonZeroCumulativeSum);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use p3_baby_bear::BabyBear;
    use p3_challenger::{HashChallenger, SerializingChallenger32};
    use p3_commit::ExtensionMmcs;
    use p3_dft::Radix2DitParallel;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriConfig, TwoAdicFriPcs};
    use p3_keccak::{Keccak256Hash, KeccakF};

    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_symmetric::{
        CompressionFunctionFromHasher, PseudoCompressionFunction, SerializingHasher32,
        TruncatedPermutation,
    };
    use p3_uni_stark::StarkConfig;
    use p3_util::log2_ceil_usize;
    use rand::random;

    fn generate_digests(leaf_hashes: Vec<[u8; 32]>) -> Vec<Vec<[u8; 32]>> {
        let keccak = TruncatedPermutation::new(KeccakF {});
        let mut digests = vec![leaf_hashes];

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
        type Val = BabyBear;
        type Challenge = BinomialExtensionField<Val, 4>;

        type ByteHash = Keccak256Hash;
        type FieldHash = SerializingHasher32<ByteHash>;
        let byte_hash = ByteHash {};
        let field_hash = FieldHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<u8, ByteHash, 2, 32>;
        let compress = MyCompress::new(byte_hash);

        type ValMmcs = FieldMerkleTreeMmcs<Val, u8, FieldHash, MyCompress, 32>;
        let val_mmcs = ValMmcs::new(field_hash, compress.clone());

        type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
        let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

        type Dft = Radix2DitParallel;
        let dft = Dft {};

        type Challenger = SerializingChallenger32<Val, HashChallenger<u8, ByteHash, 32>>;

        let fri_config = FriConfig {
            log_blowup: 2,
            num_queries: 42,
            proof_of_work_bits: 16,
            mmcs: challenge_mmcs,
        };
        type Pcs = TwoAdicFriPcs<Val, Dft, ValMmcs, ChallengeMmcs>;
        let pcs = Pcs::new(log2_ceil_usize(256), dft, val_mmcs, fri_config);

        type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;
        let config = MyConfig::new(pcs);

        const HEIGHT: usize = 3;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32))
            .map(|_| [0; 32])
            .map(|_| random())
            .collect::<Vec<_>>();
        let digests = generate_digests(leaf_hashes);

        let leaf_index = 0;
        // let machine = Machine::new(merkle_tree.digest_layers, leaf_index);
        let machine = Machine::new(digests, leaf_index);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        let proof = machine.prove(&config, &mut challenger);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        machine.verify(&config, &proof, &mut challenger)
    }
}

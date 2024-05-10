use itertools::{izip, Itertools};
use p3_air::{Air, AirBuilder, BaseAir};
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{AbstractExtensionField, AbstractField, Field, PrimeField64};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_maybe_rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator};
use p3_uni_stark::{get_log_quotient_degree, PackedChallenge, StarkGenericConfig, Val};
use p3_util::log2_strict_usize;
use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
};
use tracing::instrument;

#[cfg(feature = "debug-trace")]
use std::error::Error;

use crate::{
    check_constraints::{check_constraints, check_cumulative_sums},
    chip::{Chip, MachineChip},
    error::VerificationError,
    interaction::{Interaction, InteractionType},
    keccak_permute::KeccakPermuteChip,
    keccak_sponge::{keccakf_u8s, KeccakSpongeChip, KeccakSpongeOp, KECCAK_RATE_BYTES},
    memory::{MemoryChip, MemoryOp, OperationKind},
    merkle_tree::MerkleTreeChip,
    permutation::generate_permutation_trace,
    proof::{AdjacentOpenedValues, ChipProof, Commitments, MachineProof, OpenedValues},
    quotient::quotient_values,
    range::RangeCheckerChip,
    verify::verify_constraints,
    xor::XorChip,
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
    // TODO: Proper execution function for the machine that minimizes redundant computation
    //       Store logs/events during execution first and then generate the traces
    pub fn new(preimage_bytes: Vec<u8>, digests: Vec<Vec<[u8; 32]>>, leaf_index: usize) -> Self {
        let leaf = digests[0][leaf_index];

        let height = digests.len() - 1;
        let siblings = (0..height)
            .map(|i| digests[i][(leaf_index >> i) ^ 1])
            .collect::<Vec<[u8; 32]>>();
        let mut keccak_inputs = (0..height)
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
                        .collect_vec()
                        .as_slice(),
                );
                input[4..8].copy_from_slice(
                    right
                        .chunks_exact(8)
                        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                        .collect_vec()
                        .as_slice(),
                );
                (input, true)
            })
            .collect_vec();

        let merkle_tree_chip = MerkleTreeChip {
            leaves: vec![leaf],
            leaf_indices: vec![leaf_index],
            siblings: vec![siblings],
        };

        let keccak_sponge_chip = KeccakSpongeChip {
            inputs: vec![KeccakSpongeOp {
                timestamp: 0,
                addr: 0,
                input: preimage_bytes.clone(),
            }],
        };

        let memory_ops = preimage_bytes
            .iter()
            .enumerate()
            .map(|(i, &b)| MemoryOp {
                addr: i as u32,
                // TODO: Use proper timestamp
                timestamp: 0,
                value: b,
                kind: OperationKind::Read,
            })
            .collect_vec();
        let memory_chip = MemoryChip {
            operations: memory_ops.clone(),
        };

        let preimage_len = preimage_bytes.len();

        let mut padded_preimage = preimage_bytes.clone();
        let padding_len = KECCAK_RATE_BYTES - (preimage_len % KECCAK_RATE_BYTES);
        padded_preimage.resize(preimage_len + padding_len, 0);
        padded_preimage[preimage_len] = 1;
        *padded_preimage.last_mut().unwrap() |= 0b10000000;

        let mut xor_inputs = Vec::new();

        let mut state = [0u8; 200];
        let keccak_inputs_full = padded_preimage
            .chunks(KECCAK_RATE_BYTES)
            .map(|b| {
                state[..KECCAK_RATE_BYTES]
                    .chunks(4)
                    .zip(b.chunks(4))
                    .for_each(|(s, b)| {
                        xor_inputs.push((b.try_into().unwrap(), s.try_into().unwrap()));
                    });
                state[..KECCAK_RATE_BYTES]
                    .iter_mut()
                    .zip(b.iter())
                    .for_each(|(s, b)| {
                        *s ^= *b;
                    });
                let input: [u64; 25] = state
                    .chunks_exact(8)
                    .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                    .collect_vec()
                    .try_into()
                    .unwrap();

                keccakf_u8s(&mut state);
                input
            })
            .collect_vec();
        keccak_inputs.extend(keccak_inputs_full.into_iter().map(|input| (input, false)));

        let keccak_permute_chip = KeccakPermuteChip {
            inputs: keccak_inputs,
        };

        let mut range_counts = BTreeMap::new();
        // TODO: This is wrong, should be just the preimage
        for byte in padded_preimage {
            range_counts
                .entry(byte as u32)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
        for (i, op) in memory_ops.iter().enumerate() {
            let diff = if i > 0 {
                let op_prev = &memory_ops[i - 1];
                if op.addr == op_prev.addr {
                    op.timestamp - op_prev.timestamp
                } else {
                    op.addr - op_prev.addr - 1
                }
            } else {
                0
            };
            let diff_limb_lo = diff % (1 << 8);
            let diff_limb_md = (diff >> 8) % (1 << 8);
            let diff_limb_hi = (diff >> 16) % (1 << 8);

            range_counts
                .entry(diff_limb_lo)
                .and_modify(|c| *c += 1)
                .or_insert(1);
            range_counts
                .entry(diff_limb_md)
                .and_modify(|c| *c += 1)
                .or_insert(1);
            range_counts
                .entry(diff_limb_hi)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }

        let range_chip = RangeCheckerChip {
            count: range_counts,
        };

        let xor_chip = XorChip {
            operations: xor_inputs,
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

    // TODO: Move main trace generation outside
    #[instrument(skip_all)]
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
        // TODO: Move to ProvingKey
        let preprocessed_traces =
            tracing::info_span!("generate preprocessed traces").in_scope(|| {
                self.chips()
                    .par_iter()
                    .map(|chip| chip.preprocessed_trace())
                    .collect_vec()
            });
        let preprocessed_widths = preprocessed_traces
            .iter()
            .map(|mt| mt.as_ref().map(|trace| trace.width()).unwrap_or_default())
            .collect_vec();
        let (preprocessed_indices, preprocessed_traces_packed) = preprocessed_traces.iter().fold(
            (vec![], vec![]),
            |(mut indices, mut traces), trace| {
                if let Some(trace) = trace {
                    indices.push(Some(traces.len()));
                    traces.push(trace.clone());
                } else {
                    indices.push(None);
                }
                (indices, traces)
            },
        );

        let preprocessed_domains = preprocessed_traces_packed
            .iter()
            .map(|trace| {
                let degree = trace.height();
                pcs.natural_domain_for_degree(degree)
            })
            .collect_vec();
        let (preprocessed_commit, preprocessed_data) =
            tracing::info_span!("commit to preprocessed traces").in_scope(|| {
                pcs.commit(
                    std::iter::zip(
                        preprocessed_domains.clone(),
                        preprocessed_traces_packed.clone(),
                    )
                    .collect_vec(),
                )
            });
        challenger.observe(preprocessed_commit.clone());

        // 2. Generate and commit to main trace
        let main_traces = tracing::info_span!("generate main traces").in_scope(|| {
            self.chips()
                .par_iter()
                .map(|chip| chip.generate_trace())
                .collect_vec()
        });
        let main_degrees = main_traces.iter().map(|trace| trace.height()).collect_vec();
        // TODO: Handle empty traces
        // let (main_traces, main_degrees): (Vec<_>, Vec<_>) = main_traces
        //     .into_iter()
        //     .zip(main_degrees.iter())
        //     .filter(|(_, &degree)| degree > 0)
        //     .unzip();

        let main_domains = main_degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect_vec();

        let (main_commit, main_data) =
            tracing::info_span!("commit to main traces").in_scope(|| {
                pcs.commit(std::iter::zip(main_domains.clone(), main_traces.clone()).collect_vec())
            });
        challenger.observe(main_commit.clone());

        // 3. Generate and commit to permutation trace
        let mut perm_challenges = Vec::new();
        for _ in 0..2 {
            perm_challenges.push(challenger.sample_ext_element::<SC::Challenge>());
        }
        let packed_perm_challenges = perm_challenges
            .iter()
            .map(|c| PackedChallenge::<SC>::from_f(*c))
            .collect_vec();

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
                .collect_vec()
        });
        let (perm_indices, perm_traces_packed) =
            perm_traces
                .iter()
                .fold((vec![], vec![]), |(mut indices, mut traces), trace| {
                    if let Some(trace) = trace {
                        indices.push(Some(traces.len()));
                        traces.push(trace.clone());
                    } else {
                        indices.push(None);
                    }
                    (indices, traces)
                });
        // TODO: Assert equal to main trace degrees?
        let perm_domains = perm_traces_packed
            .iter()
            .map(|trace| {
                let degree = trace.height();
                pcs.natural_domain_for_degree(degree)
            })
            .collect_vec();
        let (perm_commit, perm_data) = tracing::info_span!("commit to permutation traces")
            .in_scope(|| {
                let flattened_perm_traces = perm_traces_packed
                    .iter()
                    .map(|trace| trace.flatten_to_base())
                    .collect_vec();
                pcs.commit(
                    std::iter::zip(perm_domains.clone(), flattened_perm_traces).collect_vec(),
                )
            });
        challenger.observe(perm_commit.clone());
        let alpha: SC::Challenge = challenger.sample_ext_element();
        let cumulative_sums = perm_traces
            .iter()
            .map(|mt| {
                mt.as_ref()
                    .map(|trace| *trace.row_slice(trace.height() - 1).last().unwrap())
            })
            .collect_vec();

        // 4. Verify constraints
        #[cfg(feature = "debug-trace")]
        let _ = self.write_traces_to_file::<SC>(
            "trace.xlsx",
            preprocessed_traces.as_slice(),
            main_traces.as_slice(),
            perm_traces.as_slice(),
        );
        #[cfg(debug_assertions)]
        for (chip, preprocessed_trace, main_trace, perm_trace, &cumulative_sum) in izip!(
            self.chips(),
            preprocessed_traces.iter(),
            main_traces.iter(),
            perm_traces.iter(),
            cumulative_sums.iter()
        ) {
            check_constraints::<_, SC>(
                chip,
                preprocessed_trace,
                main_trace,
                perm_trace,
                &perm_challenges,
                cumulative_sum,
                &[],
            );
        }
        #[cfg(debug_assertions)]
        check_cumulative_sums(&perm_traces[..]);

        // 5. Generate and commit to quotient traces
        let log_degrees = main_degrees
            .iter()
            .map(|&d| log2_strict_usize(d))
            .collect_vec();
        let log_quotient_degrees = self
            .chips()
            .iter()
            .zip(preprocessed_widths)
            .map(|(&chip, prep_width)| {
                let min_degree = if <ChipType as Chip<Val<SC>>>::has_interactions(chip) {
                    1
                } else {
                    0
                };
                get_log_quotient_degree::<Val<SC>, _>(chip, prep_width, 0).max(1)
            })
            .collect_vec();
        let quotient_degrees = log_quotient_degrees.iter().map(|d| 1 << d).collect_vec();
        let quotient_domains = main_domains
            .iter()
            .zip(log_degrees.iter())
            .zip(log_quotient_degrees.iter())
            .map(|((domain, log_degree), log_quotient_degree)| {
                domain.create_disjoint_domain(1 << (log_degree + log_quotient_degree))
            })
            .collect_vec();

        let quotient_values = quotient_domains
            .clone()
            .into_par_iter()
            .zip(self.chips().par_iter())
            .enumerate()
            .map(|(i, (quotient_domain, &chip))| {
                let preprocessed_trace_on_quotient_domains = preprocessed_indices[i]
                    .map(|index| {
                        pcs.get_evaluations_on_domain(&preprocessed_data, index, quotient_domain)
                            .to_row_major_matrix()
                    })
                    .unwrap_or(RowMajorMatrix::new_col(vec![
                        Val::<SC>::zero();
                        quotient_domain.size()
                    ]));
                let main_trace_on_quotient_domains = pcs
                    .get_evaluations_on_domain(&main_data, i, quotient_domain)
                    .to_row_major_matrix();
                let perm_trace_on_quotient_domains = perm_indices[i]
                    .map(|index| {
                        pcs.get_evaluations_on_domain(&perm_data, index, quotient_domain)
                            .to_row_major_matrix()
                    })
                    .unwrap_or(RowMajorMatrix::new_col(vec![
                        Val::<SC>::zero();
                        quotient_domain.size()
                    ]));
                quotient_values::<SC, _, _>(
                    chip,
                    cumulative_sums[i],
                    main_domains[i],
                    quotient_domain,
                    preprocessed_trace_on_quotient_domains,
                    main_trace_on_quotient_domains,
                    perm_trace_on_quotient_domains,
                    &packed_perm_challenges,
                    alpha,
                )
            })
            .collect_vec();
        let quotient_chunks = quotient_domains
            .clone()
            .into_iter()
            .zip(quotient_degrees.clone())
            .zip(quotient_values)
            .map(|((domain, degree), values)| {
                let quotient_flat = RowMajorMatrix::new_col(values).flatten_to_base();
                domain.split_evals(degree, quotient_flat)
            })
            .collect_vec();
        let qc_domains = quotient_domains
            .into_iter()
            .zip(quotient_degrees.clone())
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
                        .zip(quotient_chunks.into_iter().flatten())
                        .collect_vec(),
                )
            });
        challenger.observe(quotient_commit.clone());

        let commitments = Commitments {
            main_trace: main_commit,
            perm_trace: perm_commit,
            quotient_chunks: quotient_commit,
        };

        let zeta: SC::Challenge = challenger.sample_ext_element();
        let preprocessed_opening_points = preprocessed_domains
            .iter()
            .map(|domain| vec![zeta, domain.next_point(zeta).unwrap()])
            .collect_vec();
        let perm_opening_points = perm_domains
            .iter()
            .map(|domain| vec![zeta, domain.next_point(zeta).unwrap()])
            .collect_vec();
        let main_opening_points = main_domains
            .iter()
            .map(|domain| vec![zeta, domain.next_point(zeta).unwrap()])
            .collect_vec();
        // open every chunk at zeta
        let quotient_opening_points = quotient_degrees
            .iter()
            .flat_map(|&quotient_degree| (0..quotient_degree).map(|_| vec![zeta]).collect_vec())
            .collect_vec();

        let (opening_values, opening_proof) = pcs.open(
            vec![
                (&preprocessed_data, preprocessed_opening_points),
                (&main_data, main_opening_points),
                (&perm_data, perm_opening_points),
                (&quotient_data, quotient_opening_points),
            ],
            challenger,
        );
        let [preprocessed_openings, main_openings, perm_openings, quotient_openings] =
            opening_values
                .try_into()
                .expect("Should have 3 rounds of openings");
        let preprocessed_openings = preprocessed_indices
            .iter()
            .map(|index| index.map(|index| preprocessed_openings[index].clone()))
            .collect_vec();
        let perm_openings = perm_indices
            .iter()
            .map(|index| index.map(|index| perm_openings[index].clone()))
            .collect_vec();
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

        let chip_proofs = izip!(
            log_degrees,
            preprocessed_openings,
            main_openings,
            perm_openings,
            quotient_openings,
            cumulative_sums,
        )
        .map(
            |(log_degree, preprocessed, main, perm, quotient, cumulative_sum)| {
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
                    degree_bits: log_degree,
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
        challenger: &mut SC::Challenger,
    ) -> Result<(), VerificationError>
    where
        Val<SC>: PrimeField64,
    {
        // TODO: Move to proving, verifiying key
        let pcs = config.pcs();

        let preprocessed_traces =
            tracing::info_span!("generate preprocessed traces").in_scope(|| {
                self.chips()
                    .par_iter()
                    .map(|chip| chip.preprocessed_trace())
                    .collect_vec()
            });
        let preprocessed_widths = preprocessed_traces
            .iter()
            .map(|mt| mt.as_ref().map(|trace| trace.width()).unwrap_or_default())
            .collect_vec();
        let preprocessed_traces_packed = preprocessed_traces.into_iter().flatten().collect_vec();
        let preprocessed_domains = preprocessed_traces_packed
            .iter()
            .map(|trace| {
                let degree = trace.height();
                pcs.natural_domain_for_degree(degree)
            })
            .collect_vec();
        let (preprocessed_commit, _preprocessed_data) =
            tracing::info_span!("commit to preprocessed traces").in_scope(|| {
                pcs.commit(
                    std::iter::zip(
                        preprocessed_domains.clone(),
                        preprocessed_traces_packed.clone(),
                    )
                    .collect_vec(),
                )
            });
        challenger.observe(preprocessed_commit.clone());

        let MachineProof {
            commitments,
            opening_proof,
            chip_proofs,
        } = proof;
        let log_degrees = chip_proofs
            .iter()
            .map(|chip_proof| chip_proof.degree_bits)
            .collect_vec();

        let degrees = log_degrees
            .iter()
            .map(|degree_bits| 1 << degree_bits)
            .collect_vec();
        let log_quotient_degrees = self
            .chips()
            .iter()
            .zip(preprocessed_widths)
            .map(|(&chip, prep_width)| {
                get_log_quotient_degree::<Val<SC>, _>(chip, prep_width, 0).max(1)
            })
            .collect_vec();
        let quotient_degrees = log_quotient_degrees
            .iter()
            .map(|&log_degree| 1 << log_degree)
            .collect_vec();

        let main_domains = degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect_vec();
        let quotient_domains = main_domains
            .iter()
            .zip(log_degrees.iter())
            .zip(log_quotient_degrees.iter())
            .map(|((domain, log_degree), log_quotient_degree)| {
                domain.create_disjoint_domain(1 << (log_degree + log_quotient_degree))
            })
            .collect_vec();
        let quotient_chunks_domains = quotient_domains
            .into_iter()
            .zip(quotient_degrees.clone())
            .map(|(quotient_domain, quotient_degree)| {
                quotient_domain.split_domains(quotient_degree)
            })
            .collect_vec();

        let air_widths = self
            .chips()
            .iter()
            .map(|chip| <ChipType as BaseAir<Val<SC>>>::width(chip))
            .collect_vec();

        // TODO: Add preprocessed and permutation size check
        let valid_shape =
            chip_proofs
                .iter()
                .zip(air_widths.iter())
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
        let mut perm_challenges = Vec::new();
        for _ in 0..2 {
            perm_challenges.push(challenger.sample_ext_element::<SC::Challenge>());
        }
        challenger.observe(commitments.perm_trace.clone());
        let alpha = challenger.sample_ext_element::<SC::Challenge>();
        challenger.observe(commitments.quotient_chunks.clone());

        let zeta: SC::Challenge = challenger.sample_ext_element();
        let preprocessed_domains_and_openings = chip_proofs
            .iter()
            .flat_map(|proof| proof.opened_values.preprocessed.as_ref())
            .zip(preprocessed_domains.iter())
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

        pcs.verify(
            vec![
                (preprocessed_commit, preprocessed_domains_and_openings),
                (commitments.main_trace.clone(), main_domains_and_openings),
                (commitments.perm_trace.clone(), perm_domains_and_openings),
                (
                    commitments.quotient_chunks.clone(),
                    quotient_chunks_domains_and_openings,
                ),
            ],
            opening_proof,
            challenger,
        )
        .map_err(|_| VerificationError::InvalidOpeningArgument)?;

        for (qc_domains, chip_proof, &main_domain, &chip) in izip!(
            quotient_chunks_domains.iter(),
            chip_proofs.iter(),
            main_domains.iter(),
            self.chips().iter()
        ) {
            verify_constraints::<SC, _>(
                chip,
                &chip_proof.opened_values,
                chip_proof.cumulative_sum,
                main_domain,
                qc_domains,
                zeta,
                alpha,
                perm_challenges.as_slice(),
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
        Val<SC>: PrimeField64,
    {
        use rust_xlsxwriter::Workbook;

        let mut workbook = Workbook::new();
        for (chip, preprocessed_trace, main_trace, perm_trace) in
            izip!(self.chips(), preprocessed_traces, main_traces, perm_traces)
        {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(format!("{}", chip))?;
            chip.write_traces_to_worksheet(worksheet, preprocessed_trace, main_trace, perm_trace)?;
        }

        workbook.save(path)?;

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
        let pcs = Pcs::new(dft, val_mmcs, fri_config);

        type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;
        let config = MyConfig::new(pcs);

        const NUM_BYTES: usize = 1000;
        let preimage = (0..NUM_BYTES).map(|_| random()).collect_vec();

        const HEIGHT: usize = 8;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32)).map(|_| random()).collect_vec();
        let digests = generate_digests(&leaf_hashes);

        let leaf_index = thread_rng().gen_range(0..leaf_hashes.len());
        let machine = Machine::new(preimage, digests, leaf_index);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        let proof = machine.prove(&config, &mut challenger);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        machine.verify(&config, &proof, &mut challenger)
    }
}

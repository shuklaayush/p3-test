use itertools::{izip, Itertools};
use p3_air::{Air, AirBuilder, BaseAir};
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{AbstractExtensionField, AbstractField, Field, PrimeField64};
use p3_matrix::{dense::RowMajorMatrix, Matrix, MatrixRowSlices};
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
    keccak_sponge::{keccakf_u8s, KeccakSpongeChip, KECCAK_RATE_BYTES},
    merkle_tree::MerkleTreeChip,
    permutation::generate_permutation_trace,
    proof::{ChipProof, Commitments, MachineProof, OpenedValues},
    quotient::quotient_values,
    range::RangeCheckerChip,
    verify::verify_constraints,
    xor::XorChip,
};

pub enum ChipType {
    KeccakPermute(KeccakPermuteChip),
    KeccakSponge(KeccakSpongeChip),
    MerkleTree(MerkleTreeChip),
    Range8(RangeCheckerChip<256>),
    Xor(XorChip),
}

impl Display for ChipType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ChipType::KeccakPermute(_) => write!(f, "KeccakPermute"),
            ChipType::KeccakSponge(_) => write!(f, "KeccakSponge"),
            ChipType::MerkleTree(_) => write!(f, "MerkleTree"),
            ChipType::Range8(_) => write!(f, "Range8"),
            ChipType::Xor(_) => write!(f, "Xor"),
        }
    }
}

// TODO: Write a proc_macro for enum dispatch
impl<F: Field> BaseAir<F> for ChipType {
    fn width(&self) -> usize {
        match self {
            ChipType::KeccakPermute(chip) => <KeccakPermuteChip as BaseAir<F>>::width(chip),
            ChipType::KeccakSponge(chip) => <KeccakSpongeChip as BaseAir<F>>::width(chip),
            ChipType::MerkleTree(chip) => <MerkleTreeChip as BaseAir<F>>::width(chip),
            ChipType::Range8(chip) => <RangeCheckerChip<256> as BaseAir<F>>::width(chip),
            ChipType::Xor(chip) => <XorChip as BaseAir<F>>::width(chip),
        }
    }

    fn preprocessed_trace(&self) -> Option<RowMajorMatrix<F>> {
        match self {
            ChipType::KeccakPermute(chip) => chip.preprocessed_trace(),
            ChipType::KeccakSponge(chip) => chip.preprocessed_trace(),
            ChipType::MerkleTree(chip) => chip.preprocessed_trace(),
            ChipType::Range8(chip) => chip.preprocessed_trace(),
            ChipType::Xor(chip) => chip.preprocessed_trace(),
        }
    }
}

impl<AB: AirBuilder> Air<AB> for ChipType {
    fn eval(&self, builder: &mut AB) {
        match self {
            ChipType::KeccakPermute(chip) => chip.eval(builder),
            ChipType::KeccakSponge(chip) => chip.eval(builder),
            ChipType::MerkleTree(chip) => chip.eval(builder),
            ChipType::Range8(chip) => chip.eval(builder),
            ChipType::Xor(chip) => chip.eval(builder),
        }
    }
}

impl<F: PrimeField64> Chip<F> for ChipType {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        match self {
            ChipType::KeccakPermute(chip) => chip.generate_trace(),
            ChipType::KeccakSponge(chip) => chip.generate_trace(),
            ChipType::MerkleTree(chip) => chip.generate_trace(),
            ChipType::Range8(chip) => chip.generate_trace(),
            ChipType::Xor(chip) => chip.generate_trace(),
        }
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        match self {
            ChipType::KeccakPermute(chip) => chip.sends(),
            ChipType::KeccakSponge(chip) => chip.sends(),
            ChipType::MerkleTree(chip) => chip.sends(),
            ChipType::Range8(chip) => chip.sends(),
            ChipType::Xor(chip) => chip.sends(),
        }
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        match self {
            ChipType::KeccakPermute(chip) => chip.receives(),
            ChipType::KeccakSponge(chip) => chip.receives(),
            ChipType::MerkleTree(chip) => chip.receives(),
            ChipType::Range8(chip) => chip.receives(),
            ChipType::Xor(chip) => chip.receives(),
        }
    }

    fn all_interactions(&self) -> Vec<(Interaction<F>, InteractionType)> {
        match self {
            ChipType::KeccakPermute(chip) => chip.all_interactions(),
            ChipType::KeccakSponge(chip) => chip.all_interactions(),
            ChipType::MerkleTree(chip) => chip.all_interactions(),
            ChipType::Range8(chip) => chip.all_interactions(),
            ChipType::Xor(chip) => chip.all_interactions(),
        }
    }

    #[cfg(feature = "debug-trace")]
    fn main_headers(&self) -> Vec<String> {
        match self {
            ChipType::KeccakPermute(chip) => <KeccakPermuteChip as Chip<F>>::main_headers(chip),
            ChipType::KeccakSponge(chip) => <KeccakSpongeChip as Chip<F>>::main_headers(chip),
            ChipType::MerkleTree(chip) => <MerkleTreeChip as Chip<F>>::main_headers(chip),
            ChipType::Range8(chip) => <RangeCheckerChip<256> as Chip<F>>::main_headers(chip),
            ChipType::Xor(chip) => <XorChip as Chip<F>>::main_headers(chip),
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
            ChipType::KeccakSponge(chip) => {
                <KeccakSpongeChip as MachineChip<SC>>::trace_width(chip)
            }
            ChipType::MerkleTree(chip) => <MerkleTreeChip as MachineChip<SC>>::trace_width(chip),
            ChipType::Range8(chip) => <RangeCheckerChip<256> as MachineChip<SC>>::trace_width(chip),
            ChipType::Xor(chip) => <XorChip as MachineChip<SC>>::trace_width(chip),
        }
    }
}

pub struct Machine {
    keccak_permute_chip: ChipType,
    keccak_sponge_chip: ChipType,
    merkle_tree_chip: ChipType,
    range_chip: ChipType,
    xor_chip: ChipType,
}

impl Machine {
    pub fn chips(&self) -> Vec<&ChipType> {
        vec![
            &self.keccak_permute_chip,
            &self.keccak_sponge_chip,
            &self.merkle_tree_chip,
            &self.range_chip,
            &self.xor_chip,
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
}

impl Machine {
    // TODO: Proper execution function for the machine that minimizes redundant computation
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
            siblings: vec![siblings.try_into().unwrap()],
        };

        let keccak_sponge_chip = KeccakSpongeChip {
            inputs: vec![preimage_bytes.clone()],
        };

        let preimage_len = preimage_bytes.len();

        let mut padded_preimage = preimage_bytes.clone();
        let padding_len = KECCAK_RATE_BYTES - (preimage_len % KECCAK_RATE_BYTES);
        padded_preimage.resize(preimage_len + padding_len, 0);
        padded_preimage[preimage_len] = 1;
        *padded_preimage.last_mut().unwrap() |= 0b10000000;

        let mut xor_inputs = Vec::new();

        let mut state = [0u8; 200];
        keccak_inputs.push(
            padded_preimage
                .chunks(KECCAK_RATE_BYTES)
                .flat_map(|b| {
                    state[..KECCAK_RATE_BYTES]
                        .chunks(4)
                        .zip(b.chunks(4))
                        .for_each(|(s, b)| {
                            xor_inputs.push((s.try_into().unwrap(), b.try_into().unwrap()));
                        });
                    state[..KECCAK_RATE_BYTES]
                        .iter_mut()
                        .zip(b.iter())
                        .for_each(|(s, b)| {
                            *s ^= *b;
                        });
                    let input = state
                        .chunks_exact(8)
                        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
                        .collect_vec();

                    keccakf_u8s(&mut state);
                    input
                })
                .collect_vec()
                .try_into()
                .map(|input| (input, false))
                .unwrap(),
        );

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
        let maybe_preprocessed_traces = tracing::info_span!("generate preprocessed traces")
            .in_scope(|| {
                self.chips()
                    .par_iter()
                    .map(|chip| chip.preprocessed_trace())
                    .collect_vec()
            });
        let (preprocessed_indices, preprocessed_traces) = maybe_preprocessed_traces
            .clone()
            .into_iter()
            .fold((vec![], vec![]), |(mut indices, mut traces), trace| {
                if let Some(trace) = trace {
                    indices.push(Some(traces.len()));
                    traces.push(trace);
                } else {
                    indices.push(None);
                }
                (indices, traces)
            });

        let preprocessed_degrees = preprocessed_traces
            .iter()
            .map(|trace| trace.height())
            .collect_vec();
        let preprocessed_domains = preprocessed_degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect_vec();
        let (preprocessed_commit, preprocessed_data) =
            tracing::info_span!("commit to preprocessed traces").in_scope(|| {
                pcs.commit(
                    std::iter::zip(preprocessed_domains.clone(), preprocessed_traces.clone())
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
            .collect::<Vec<_>>();

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
        // TODO: Assert equal to main trace degrees?
        let perm_degrees = perm_traces
            .iter()
            .map(|trace: &RowMajorMatrix<SC::Challenge>| trace.height())
            .collect_vec();
        let perm_domains = perm_degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect_vec();
        let (perm_commit, perm_data) = tracing::info_span!("commit to permutation traces")
            .in_scope(|| {
                let flattened_perm_traces = perm_traces
                    .iter()
                    .map(|trace| trace.flatten_to_base())
                    .collect_vec();
                pcs.commit(std::iter::zip(perm_domains, flattened_perm_traces).collect_vec())
            });
        challenger.observe(perm_commit.clone());
        let alpha: SC::Challenge = challenger.sample_ext_element();
        let cumulative_sums = perm_traces
            .iter()
            .map(|trace| *trace.row_slice(trace.height() - 1).last().unwrap())
            .collect_vec();

        // 4. Verify constraints
        #[cfg(feature = "debug-trace")]
        let _ = self.write_traces_to_file::<SC>(
            "trace.xlsx",
            maybe_preprocessed_traces.as_slice(),
            main_traces.as_slice(),
            perm_traces.as_slice(),
        );
        #[cfg(debug_assertions)]
        for (chip, main_trace, perm_trace) in
            izip!(self.chips(), main_traces.iter(), perm_traces.iter())
        {
            check_constraints::<_, SC>(chip, main_trace, perm_trace, &perm_challenges, &vec![]);
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
            .map(|&chip| get_log_quotient_degree::<Val<SC>, _>(chip, 0))
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
                let preprocessed_trace_on_quotient_domains =
                    if let Some(index) = preprocessed_indices[i] {
                        pcs.get_evaluations_on_domain(&preprocessed_data, index, quotient_domain)
                    } else {
                        RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                    };
                let main_trace_on_quotient_domains =
                    pcs.get_evaluations_on_domain(&main_data, i, quotient_domain);
                let permutation_trace_on_quotient_domains =
                    pcs.get_evaluations_on_domain(&perm_data, i, quotient_domain);
                quotient_values::<SC, _, _>(
                    chip,
                    cumulative_sums[i],
                    main_domains[i],
                    quotient_domain,
                    preprocessed_trace_on_quotient_domains,
                    main_trace_on_quotient_domains,
                    permutation_trace_on_quotient_domains,
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
                (&main_data, main_opening_points.clone()),
                (&perm_data, main_opening_points),
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
            perm_traces
        )
        .map(
            |(log_degree, preprocessed, main, perm, quotient, perm_trace)| {
                let [preprocessed_local, preprocessed_next] =
                    if let Some(preprocessed) = preprocessed {
                        preprocessed.try_into().expect("Should have 2 openings")
                    } else {
                        [vec![], vec![]]
                    };
                let [main_local, main_next] = main.try_into().expect("Should have 2 openings");
                let [perm_local, perm_next] = perm.try_into().expect("Should have 2 openings");
                let quotient_chunks = quotient
                    .into_iter()
                    .map(|mut chunk| chunk.remove(0))
                    .collect_vec();
                let opened_values = OpenedValues {
                    preprocessed_local,
                    preprocessed_next,
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
            .map(|&chip| get_log_quotient_degree::<Val<SC>, _>(chip, 0))
            .collect_vec();
        let quotient_degrees = log_quotient_degrees
            .iter()
            .map(|&log_degree| 1 << log_degree)
            .collect_vec();

        let pcs = config.pcs();
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

        let maybe_preprocessed_traces = tracing::info_span!("generate preprocessed traces")
            .in_scope(|| {
                self.chips()
                    .par_iter()
                    .map(|chip| chip.preprocessed_trace())
                    .collect_vec()
            });
        let (preprocessed_indices, preprocessed_traces) = maybe_preprocessed_traces
            .into_iter()
            .fold((vec![], vec![]), |(mut indices, mut traces), trace| {
                if let Some(trace) = trace {
                    indices.push(Some(traces.len()));
                    traces.push(trace);
                } else {
                    indices.push(None);
                }
                (indices, traces)
            });
        let preprocessed_degrees = preprocessed_traces
            .iter()
            .map(|trace| trace.height())
            .collect_vec();
        let preprocessed_domains = preprocessed_degrees
            .iter()
            .map(|&degree| pcs.natural_domain_for_degree(degree))
            .collect_vec();
        let (preprocessed_commit, _preprocessed_data) =
            tracing::info_span!("commit to preprocessed traces").in_scope(|| {
                pcs.commit(
                    std::iter::zip(preprocessed_domains.clone(), preprocessed_traces.clone())
                        .collect_vec(),
                )
            });
        challenger.observe(preprocessed_commit.clone());

        challenger.observe(commitments.main_trace.clone());
        let mut perm_challenges = Vec::new();
        for _ in 0..2 {
            perm_challenges.push(challenger.sample_ext_element::<SC::Challenge>());
        }
        challenger.observe(commitments.perm_trace.clone());
        let alpha = challenger.sample_ext_element::<SC::Challenge>();
        challenger.observe(commitments.quotient_chunks.clone());

        let zeta: SC::Challenge = challenger.sample_ext_element();
        let preprocessed_domains_and_openings = preprocessed_domains
            .iter()
            .zip(
                chip_proofs
                    .iter()
                    .enumerate()
                    .filter_map(|(i, chip_proof)| {
                        if preprocessed_indices[i].is_some() {
                            Some(chip_proof)
                        } else {
                            None
                        }
                    }),
            )
            .map(|(&domain, proof)| {
                (
                    domain,
                    vec![
                        (zeta, proof.opened_values.preprocessed_local.clone()),
                        (
                            domain.next_point(zeta).unwrap(),
                            proof.opened_values.preprocessed_next.clone(),
                        ),
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
                        (zeta, proof.opened_values.trace_local.clone()),
                        (
                            domain.next_point(zeta).unwrap(),
                            proof.opened_values.trace_next.clone(),
                        ),
                    ],
                )
            })
            .collect_vec();

        let perm_domains_and_openings = main_domains
            .iter()
            .zip(chip_proofs.iter())
            .map(|(&domain, proof)| {
                (
                    domain,
                    vec![
                        (zeta, proof.opened_values.permutation_local.clone()),
                        (
                            domain.next_point(zeta).unwrap(),
                            proof.opened_values.permutation_next.clone(),
                        ),
                    ],
                )
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
            .map(|chip_proof| chip_proof.cumulative_sum)
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
        perm_traces: &[RowMajorMatrix<SC::Challenge>],
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
    use p3_util::log2_ceil_usize;
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
        let pcs = Pcs::new(log2_ceil_usize(256), dft, val_mmcs, fri_config);

        type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;
        let config = MyConfig::new(pcs);

        const HEIGHT: usize = 3;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32)).map(|_| random()).collect_vec();
        let digests = generate_digests(&leaf_hashes);

        let leaf_index = thread_rng().gen_range(0..leaf_hashes.len());
        let machine = Machine::new(vec![0], digests, leaf_index);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        let proof = machine.prove(&config, &mut challenger);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        machine.verify(&config, &proof, &mut challenger)
    }
}

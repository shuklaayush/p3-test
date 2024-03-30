

use p3_baby_bear::BabyBear;
use p3_challenger::{CanObserve, FieldChallenger};
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{AbstractField, PrimeField64};
use p3_keccak::Keccak256Hash;
use p3_matrix::{dense::RowMajorMatrix, Matrix, MatrixRowSlices};
use p3_maybe_rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator};
use p3_merkle_tree::FieldMerkleTree;
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
use p3_uni_stark::{get_log_quotient_degree, StarkGenericConfig, Val};
use p3_util::log2_strict_usize;
use rand::random;

use crate::{
    chip::{
        check_cumulative_sums, generate_permutation_trace, Chip, Interaction, InteractionType,
    },
    keccak_permute::KeccakPermuteChip,
    merkle_tree::MerkleTreeChip,
    proof::{ChipProof, Commitments, MachineProof, OpenedValues},
    quotient::quotient_values,
};

const HEIGHT: usize = 8;

pub enum ChipType {
    KeccakPermute(KeccakPermuteChip),
    MerkleTree(MerkleTreeChip<HEIGHT>),
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
    pub fn new() -> Self {
        type Val = BabyBear;

        type ByteHash = Keccak256Hash;
        type FieldHash = SerializingHasher32<ByteHash>;
        let byte_hash = ByteHash {};
        let field_hash = FieldHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<u8, ByteHash, 2, 32>;
        let compress = MyCompress::new(byte_hash);

        const NUM_HASHES: usize = 2;
        let inputs = (0..NUM_HASHES).map(|_| random()).collect::<Vec<_>>();
        let keccak_permute_chip = KeccakPermuteChip { inputs };

        let raw_leaves = (0..2u64.pow(HEIGHT as u32))
            .map(|_| random())
            .collect::<Vec<_>>();
        let merkle_tree = FieldMerkleTree::new::<Val, u8, FieldHash, MyCompress>(
            &field_hash,
            &compress,
            vec![RowMajorMatrix::new(raw_leaves, 1)],
        );
        let leaf_index = 0;

        let leaf = merkle_tree.digest_layers[0][leaf_index];
        let siblings = (0..HEIGHT)
            .map(|i| merkle_tree.digest_layers[i][(leaf_index >> i) ^ 1])
            .collect::<Vec<[u8; 32]>>();
        let merkle_tree_chip = MerkleTreeChip::<HEIGHT> {
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
        for _ in 0..3 {
            perm_challenges.push(challenger.sample_ext_element::<SC::Challenge>());
        }

        let perm_traces = tracing::info_span!("generate permutation traces").in_scope(|| {
            self.chips()
                .into_par_iter()
                .enumerate()
                .map(|(i, chip)| match chip {
                    ChipType::KeccakPermute(chip) => generate_permutation_trace::<SC, _>(
                        chip,
                        &main_traces[i],
                        perm_challenges.clone(),
                    ),
                    ChipType::MerkleTree(chip) => generate_permutation_trace::<SC, _>(
                        chip,
                        &main_traces[i],
                        perm_challenges.clone(),
                    ),
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
        let cumulative_sums = perm_traces
            .iter()
            .map(|trace| *trace.row_slice(trace.height() - 1).last().unwrap())
            .collect::<Vec<_>>();

        // 4. Verify constraints
        // TODO: Add check_constraints that checks permutation constraints
        // #[cfg(debug_assertions)]
        // check_constraints::<Self, _, SC>(
        //     self,
        //     chip,
        //     &main_traces[0usize],
        //     &perm_traces[0usize],
        //     &perm_challenges,
        // );
        #[cfg(debug_assertions)]
        // check_constraints(self.keccak_permute_chip, &main_traces[0usize]);
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
            .map(|chip| match chip {
                ChipType::KeccakPermute(chip) => get_log_quotient_degree::<Val<SC>, _>(chip, 0),
                ChipType::MerkleTree(chip) => get_log_quotient_degree::<Val<SC>, _>(chip, 0),
            })
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

        let alpha: SC::Challenge = challenger.sample_ext_element();

        let quotient_values = quotient_domains
            .clone()
            .into_par_iter()
            .enumerate()
            .map(|(i, quotient_domain)| {
                // let ppt: Option<RowMajorMatrix<Val<SC>>> =
                //     self.keccak_permute_chip.preprocessed_trace();
                // let preprocessed_trace_lde = ppt.map(|trace| preprocessed_trace_ldes.remove(0));
                let main_trace_on_quotient_domains =
                    pcs.get_evaluations_on_domain(&main_data, i, quotient_domain);
                let permutation_trace_on_quotient_domains =
                    pcs.get_evaluations_on_domain(&perm_data, i, quotient_domain);
                match self.chips()[i] {
                    ChipType::KeccakPermute(chip) => quotient_values::<SC, _, _>(
                        chip,
                        cumulative_sums[i],
                        main_domains[i],
                        quotient_domain,
                        main_trace_on_quotient_domains,
                        permutation_trace_on_quotient_domains,
                        &perm_challenges,
                        alpha,
                    ),
                    ChipType::MerkleTree(chip) => quotient_values::<SC, _, _>(
                        chip,
                        cumulative_sums[i],
                        main_domains[i],
                        quotient_domain,
                        main_trace_on_quotient_domains,
                        permutation_trace_on_quotient_domains,
                        &perm_challenges,
                        alpha,
                    ),
                }
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
            .zip(quotient_degrees)
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
        let zeta_exp_quotient_degree = log_quotient_degrees
            .iter()
            .map(|&log_deg| vec![zeta.exp_power_of_2(log_deg)])
            .collect::<Vec<_>>();

        let prover_data_and_points = vec![
            (&main_data, zeta_and_next.clone()),
            (&perm_data, zeta_and_next),
            (&quotient_data, zeta_exp_quotient_degree),
        ];
        let (openings, opening_proof) = pcs.open(prover_data_and_points, challenger);
        let [main_openings, perm_openings, quotient_openings] = openings
            .try_into()
            .expect("Should have 3 rounds of openings");

        let chip_proofs = log_degrees
            .iter()
            .zip(main_openings)
            .zip(perm_openings)
            .zip(quotient_openings)
            .zip(perm_traces)
            .map(|((((log_degree, main), perm), quotient), perm_trace)| {
                let [main_local, main_next] = main.try_into().expect("Should have 2 openings");
                let [perm_local, perm_next] = perm.try_into().expect("Should have 2 openings");
                let [quotient_chunks] = quotient.try_into().expect("Should have 1 opening");
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
                    log_degree: *log_degree,
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

    // fn verify<SC: StarkGenericConfig>(
    //     &self,
    //     config: &SC,
    //     proof: &MachineProof<SC>,
    //     challenger: &mut SC::Challenger,
    // ) -> core::result::Result<(), ()> {
    //     let log_quotient_degrees: [usize; 2usize] = [
    //         get_log_quotient_degree(self.keccak_permute_chip()),
    //         get_log_quotient_degree(self.merkle_tree_chip()),
    //     ];
    //     let pcs = config.pcs();
    //     let chips_interactions = self
    //         .chips()
    //         .iter()
    //         .map(|chip| chip.all_interactions())
    //         .collect::<Vec<_>>();
    //     let dims = &[
    //         self.chips()
    //             .iter()
    //             .zip(proof.chip_proofs.iter())
    //             .map(|(chip, chip_proof)| Dimensions {
    //                 width: chip.trace_width(),
    //                 height: 1 << chip_proof.log_degree,
    //             })
    //             .collect::<Vec<_>>(),
    //         chips_interactions
    //             .iter()
    //             .zip(proof.chip_proofs.iter())
    //             .map(|(interactions, chip_proof)| Dimensions {
    //                 width: (interactions.len() + 1) * SC::Challenge::D,
    //                 height: 1 << chip_proof.log_degree,
    //             })
    //             .collect::<Vec<_>>(),
    //         proof
    //             .chip_proofs
    //             .iter()
    //             .zip(log_quotient_degrees)
    //             .map(|(chip_proof, log_quotient_deg)| Dimensions {
    //                 width: log_quotient_deg << SC::Challenge::D,
    //                 height: 1 << chip_proof.log_degree,
    //             })
    //             .collect::<Vec<_>>(),
    //     ];
    //     let g_subgroups: [Val<SC>; 2usize] = proof
    //         .chip_proofs
    //         .iter()
    //         .map(|chip_proof| Val::<SC>::two_adic_generator(chip_proof.log_degree))
    //         .collect::<Vec<_>>()
    //         .try_into()
    //         .unwrap();
    //     let mut main_values = vec![];
    //     let mut perm_values = vec![];
    //     let mut quotient_values = vec![];
    //     for chip_proof in proof.chip_proofs.iter() {
    //         let OpenedValues {
    //             trace_local,
    //             trace_next,
    //             permutation_local,
    //             permutation_next,
    //             quotient_chunks,
    //         } = &chip_proof.opened_values;
    //         main_values.push(vec![trace_local.clone(), trace_next.clone()]);
    //         perm_values.push(vec![permutation_local.clone(), permutation_next.clone()]);
    //         quotient_values.push(vec![quotient_chunks.clone()]);
    //     }
    //     let chips_opening_values = vec![main_values, perm_values, quotient_values];
    //     let Commitments {
    //         main_trace,
    //         perm_trace,
    //         quotient_chunks,
    //     } = &proof.commitments;
    //     challenger.observe(main_trace.clone());
    //     let mut perm_challenges = Vec::new();
    //     for _ in 0..3 {
    //         perm_challenges.push(challenger.sample_ext_element::<SC::Challenge>());
    //     }
    //     challenger.observe(perm_trace.clone());
    //     let alpha = challenger.sample_ext_element::<SC::Challenge>();
    //     challenger.observe(quotient_chunks.clone());
    //     let zeta: SC::Challenge = challenger.sample_ext_element();
    //     let zeta_and_next: [Vec<SC::Challenge>; 2usize] = g_subgroups.map(|g| vec![zeta, zeta * g]);
    //     let zeta_exp_quotient_degree: [Vec<SC::Challenge>; 2usize] =
    //         log_quotient_degrees.map(|log_deg| vec![zeta.exp_power_of_2(log_deg)]);
    //     pcs.verify(
    //         &[
    //             (main_trace.clone(), zeta_and_next.as_slice()),
    //             (perm_trace.clone(), zeta_and_next.as_slice()),
    //             (quotient_chunks.clone(), zeta_exp_quotient_degree.as_slice()),
    //         ],
    //         dims,
    //         chips_opening_values,
    //         &proof.opening_proof,
    //         &mut challenger,
    //     )
    //     .map_err(|_| ())?;
    //     verify_constraints::<Self, _, SC>(
    //         self,
    //         self.keccak_permute_chip(),
    //         &proof.chip_proofs[11usize].opened_values,
    //         proof.chip_proofs[11usize].cumulative_sum,
    //         proof.chip_proofs[11usize].log_degree,
    //         g_subgroups[11usize],
    //         zeta,
    //         alpha,
    //         &perm_challenges,
    //     )
    //     .expect(format!("Failed to verify constraints on chip {}", 0usize));
    //     verify_constraints::<Self, _, SC>(
    //         self,
    //         self.merkle_tree_chip(),
    //         &proof.chip_proofs[12usize].opened_values,
    //         proof.chip_proofs[12usize].cumulative_sum,
    //         proof.chip_proofs[12usize].log_degree,
    //         g_subgroups[12usize],
    //         zeta,
    //         alpha,
    //         &perm_challenges,
    //     )
    //     .expect(format!("Failed to verify constraints on chip {}", 1usize));
    //     let sum: SC::Challenge = proof
    //         .chip_proofs
    //         .iter()
    //         .map(|chip_proof| chip_proof.cumulative_sum)
    //         .sum();
    //     if sum != SC::Challenge::zero() {
    //         return Err(());
    //     }
    //     Ok(())
    // }
}

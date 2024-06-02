#[cfg(feature = "trace-writer")]
use alloc::boxed::Box;
#[cfg(feature = "trace-writer")]
use alloc::collections::BTreeSet;
#[cfg(feature = "trace-writer")]
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
#[cfg(feature = "trace-writer")]
use core::error::Error;

use itertools::Itertools;
use p3_air::BaseAir;
use p3_air_util::{
    check_constraints, check_cumulative_sums, get_quotient_degree,
    proof::{AdjacentOpenedValues, InteractionAirProof, OpenedValues},
};
#[cfg(feature = "trace-writer")]
use p3_air_util::{track_failing_constraints, track_failing_interactions, util::TraceEntry};
use p3_commit::{OpenedValuesForRound, Pcs, PolynomialSpace};
#[cfg(feature = "trace-writer")]
use p3_field::PrimeField32;
use p3_field::{AbstractExtensionField, AbstractField, ExtensionField, Field};
use p3_interaction::{generate_permutation_trace, NUM_PERM_CHALLENGES};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_uni_stark::{Domain, PackedChallenge, StarkGenericConfig, Val};

use crate::{
    chip::MachineChip, error::VerificationError, proof::Com, proof::PcsProverData,
    quotient::quotient_values, verify::verify_constraints,
};

#[derive(Clone)]
pub struct Trace<F, Domain>
where
    F: Field,
    Domain: PolynomialSpace,
{
    pub value: RowMajorMatrix<F>,
    pub domain: Domain,
}

impl<EF, Domain> Trace<EF, Domain>
where
    EF: Field,
    Domain: PolynomialSpace,
{
    pub fn flatten_to_base<F: Field>(&self) -> Trace<F, Domain>
    where
        EF: ExtensionField<F>,
    {
        Trace {
            value: self.value.flatten_to_base(),
            domain: self.domain,
        }
    }
}

#[derive(Clone)]
pub struct IndexedTrace<F, Domain>
where
    F: Field,
    Domain: PolynomialSpace,
{
    pub trace: Trace<F, Domain>,
    pub opening_index: usize,
}

#[derive(Clone)]
pub struct QuotientTrace<Domain>
where
    Domain: PolynomialSpace,
{
    pub traces: Vec<Trace<Domain::Val, Domain>>,
    pub opening_index: usize,
}

#[derive(Clone)]
pub struct ChipTrace<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    pub chip: C,

    pub preprocessed: Option<IndexedTrace<Val<SC>, Domain<SC>>>,
    pub main: Option<IndexedTrace<Val<SC>, Domain<SC>>>,
    pub permutation: Option<IndexedTrace<SC::Challenge, Domain<SC>>>,

    pub cumulative_sum: Option<SC::Challenge>,

    pub quotient_chunks: Option<QuotientTrace<Domain<SC>>>,
    pub quotient_degree: Option<usize>,
}

impl<SC, C> ChipTrace<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    pub fn new(chip: C) -> Self {
        Self {
            chip,
            preprocessed: None,
            main: None,
            permutation: None,
            cumulative_sum: None,
            quotient_chunks: None,
            quotient_degree: None,
        }
    }

    // TODO: Change to be just main degree
    pub fn domain(&self) -> Option<Domain<SC>> {
        match (&self.preprocessed, &self.main) {
            (Some(preprocessed), Some(main)) => {
                let preprocessed_domain = preprocessed.trace.domain;
                let main_domain = main.trace.domain;
                if main_domain.size() > preprocessed_domain.size() {
                    Some(main_domain)
                } else {
                    Some(preprocessed_domain)
                }
            }
            (Some(preprocessed), None) => Some(preprocessed.trace.domain),
            (None, Some(main)) => Some(main.trace.domain),
            (None, None) => None,
        }
    }
}

pub type MachineTrace<SC, C> = Vec<ChipTrace<SC, C>>;

pub trait MachineTraceBuilder<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn new(chips: &[C]) -> Self;
}

impl<SC, C> MachineTraceBuilder<SC, C> for MachineTrace<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn new(chips: &[C]) -> Self {
        chips
            .iter()
            .map(|chip| ChipTrace::new(chip.clone()))
            .collect_vec()
    }
}

pub trait MachineTraceLoader<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn generate_preprocessed(&mut self, pcs: &'a SC::Pcs);

    fn load_preprocessed(
        &mut self,
        pcs: &'a SC::Pcs,
        traces: &'a [Option<RowMajorMatrix<Val<SC>>>],
    );

    fn load_main(&mut self, pcs: &'a SC::Pcs, traces: Vec<Option<RowMajorMatrix<Val<SC>>>>);

    fn generate_permutation(
        &mut self,
        pcs: &'a SC::Pcs,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    );

    fn generate_quotient(
        &mut self,
        pcs: &'a SC::Pcs,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        alpha: SC::Challenge,
        public_values: &[Val<SC>],
    );
}

impl<'a, SC, C> MachineTraceLoader<'a, SC> for MachineTrace<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn generate_preprocessed(&mut self, pcs: &'a SC::Pcs) {
        let traces = self
            .iter()
            .map(|trace| trace.chip.preprocessed_trace())
            .collect_vec();
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, preprocessed) in self.iter_mut().zip_eq(traces) {
            chip_trace.preprocessed = preprocessed;
        }
    }

    fn load_preprocessed(
        &mut self,
        pcs: &'a SC::Pcs,
        traces: &'a [Option<RowMajorMatrix<Val<SC>>>],
    ) {
        let traces = load_traces::<SC, _>(pcs, traces.to_vec());
        for (chip_trace, preprocessed) in self.iter_mut().zip_eq(traces) {
            chip_trace.preprocessed = preprocessed;
        }
    }

    fn load_main(&mut self, pcs: &'a SC::Pcs, traces: Vec<Option<RowMajorMatrix<Val<SC>>>>) {
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, main) in self.iter_mut().zip_eq(traces) {
            chip_trace.main = main;
        }
    }

    fn generate_permutation(
        &mut self,
        pcs: &'a SC::Pcs,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    ) {
        let traces = self
            .iter()
            .map(|trace| {
                let preprocessed = trace
                    .preprocessed
                    .as_ref()
                    .map(|mt| mt.trace.value.as_view());
                let main = trace.main.as_ref().map(|mt| mt.trace.value.as_view());
                let interactions = trace.chip.all_interactions();

                generate_permutation_trace(&preprocessed, &main, &interactions, perm_challenges)
            })
            .collect_vec();
        let cumulative_sums = traces
            .iter()
            .map(|mt| {
                mt.as_ref().map(|trace| {
                    let row = trace.row_slice(trace.height() - 1);
                    let cumulative_sum = row.last().unwrap();
                    *cumulative_sum
                })
            })
            .collect_vec();
        let traces = load_traces::<SC, _>(pcs, traces);
        for ((chip_trace, permutation), cumulative_sum) in self
            .iter_mut()
            .zip_eq(traces.into_iter())
            .zip_eq(cumulative_sums.into_iter())
        {
            chip_trace.permutation = permutation;
            chip_trace.cumulative_sum = cumulative_sum;
        }
    }

    fn generate_quotient(
        &mut self,
        pcs: &'a SC::Pcs,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        alpha: SC::Challenge,
        public_values: &[Val<SC>],
    ) {
        let perm_challenges = perm_challenges.map(PackedChallenge::<SC>::from_f);
        let alpha = PackedChallenge::<SC>::from_f(alpha);

        let mut count = 0;
        for chip_trace in self.iter_mut() {
            let quotient_degree =
                get_quotient_degree::<Val<SC>, _>(&chip_trace.chip, public_values.len());
            let trace_domain = chip_trace.domain();

            if let Some(trace_domain) = trace_domain {
                let quotient_domain =
                    trace_domain.create_disjoint_domain(trace_domain.size() * quotient_degree);

                let preprocessed_trace_on_quotient_domains =
                    if let Some(preprocessed) = &chip_trace.preprocessed {
                        pcs.get_evaluations_on_domain(
                            preprocessed_data.as_ref().unwrap(),
                            preprocessed.opening_index,
                            quotient_domain,
                        )
                        .to_row_major_matrix()
                    } else {
                        RowMajorMatrix::new(vec![], 0)
                    };
                let main_trace_on_quotient_domains = if let Some(main) = &chip_trace.main {
                    pcs.get_evaluations_on_domain(
                        main_data.as_ref().unwrap(),
                        main.opening_index,
                        quotient_domain,
                    )
                    .to_row_major_matrix()
                } else {
                    RowMajorMatrix::new(vec![], 0)
                };
                let perm_trace_on_quotient_domains =
                    if let Some(permutation) = &chip_trace.permutation {
                        pcs.get_evaluations_on_domain(
                            permutation_data.as_ref().unwrap(),
                            permutation.opening_index,
                            quotient_domain,
                        )
                        .to_row_major_matrix()
                    } else {
                        RowMajorMatrix::new(vec![], 0)
                    };

                let cumulative_sum = chip_trace
                    .cumulative_sum
                    .map(PackedChallenge::<SC>::from_f)
                    .unwrap_or_default();

                let quotient_values = quotient_values::<SC, _, _>(
                    &chip_trace.chip,
                    trace_domain,
                    quotient_domain,
                    preprocessed_trace_on_quotient_domains,
                    main_trace_on_quotient_domains,
                    perm_trace_on_quotient_domains,
                    perm_challenges,
                    alpha,
                    cumulative_sum,
                    public_values,
                );
                let quotient_flat = RowMajorMatrix::new_col(quotient_values).flatten_to_base();

                let chunks = quotient_domain.split_evals(quotient_degree, quotient_flat);
                let chunk_domains = quotient_domain.split_domains(quotient_degree);
                let traces = chunk_domains
                    .into_iter()
                    .zip_eq(chunks.into_iter())
                    .map(|(domain, chunk)| Trace {
                        value: chunk,
                        domain,
                    })
                    .collect();

                chip_trace.quotient_degree = Some(quotient_degree);
                chip_trace.quotient_chunks = Some(QuotientTrace {
                    traces,
                    opening_index: count,
                });
                count += 1;
            }
        }
    }
}

pub trait MachineTraceCommiter<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn commit_preprocessed(&self, pcs: &'a SC::Pcs)
        -> (Option<Com<SC>>, Option<PcsProverData<SC>>);

    fn commit_main(&self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>);

    fn commit_permutation(&self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>);

    fn commit_quotient(&self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>);
}

impl<'a, SC, C> MachineTraceCommiter<'a, SC> for MachineTrace<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn commit_preprocessed(
        &self,
        pcs: &'a SC::Pcs,
    ) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .iter()
            .flat_map(|trace| {
                trace
                    .preprocessed
                    .as_ref()
                    .map(|preprocessed| preprocessed.trace.clone())
            })
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_main(&self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .iter()
            .flat_map(|trace| trace.main.as_ref().map(|main| main.trace.clone()))
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_permutation(&self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .iter()
            .flat_map(|trace| {
                trace
                    .permutation
                    .as_ref()
                    .map(|permutation| permutation.trace.flatten_to_base())
            })
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_quotient(&self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .iter()
            .flat_map(|trace| {
                trace
                    .quotient_chunks
                    .as_ref()
                    .map(|quotient| quotient.traces.clone())
            })
            .flatten()
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }
}

pub trait MachineTraceChecker<SC>
where
    SC: StarkGenericConfig,
{
    fn check_constraints(&self, perm_challenges: [SC::Challenge; 2], public_values: &[Val<SC>]);
}

impl<SC, C> MachineTraceChecker<SC> for MachineTrace<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn check_constraints(&self, perm_challenges: [SC::Challenge; 2], public_values: &[Val<SC>]) {
        for chip_trace in self.iter() {
            let preprocessed = chip_trace
                .preprocessed
                .as_ref()
                .map(|preprocessed| preprocessed.trace.value.as_view());
            let main = chip_trace
                .main
                .as_ref()
                .map(|main| main.trace.value.as_view());
            let permutation = chip_trace
                .permutation
                .as_ref()
                .map(|permutation| permutation.trace.value.as_view());
            check_constraints(
                &chip_trace.chip,
                &preprocessed,
                &main,
                &permutation,
                perm_challenges,
                chip_trace.cumulative_sum,
                public_values,
            );
        }
        let preprocessed_traces = self
            .iter()
            .map(|chip_trace| {
                chip_trace
                    .preprocessed
                    .as_ref()
                    .map(|preprocessed| preprocessed.trace.value.as_view())
            })
            .collect_vec();
        let main_traces = self
            .iter()
            .map(|chip_trace| {
                chip_trace
                    .main
                    .as_ref()
                    .map(|main| main.trace.value.as_view())
            })
            .collect_vec();
        let permutation_traces = self
            .iter()
            .map(|chip_trace| {
                chip_trace
                    .permutation
                    .as_ref()
                    .map(|permutation| permutation.trace.value.as_view())
            })
            .collect_vec();

        let airs = self
            .iter()
            .map(|chip_trace| chip_trace.chip.clone())
            .collect_vec();

        check_cumulative_sums(
            &airs,
            preprocessed_traces.as_slice(),
            main_traces.as_slice(),
            permutation_traces.as_slice(),
        );
    }
}

#[cfg(feature = "trace-writer")]
pub trait MachineTraceDebugger<SC>
where
    SC: StarkGenericConfig,
    Val<SC>: PrimeField32,
{
    // TODO: Move to separate trait
    fn track_failing_constraints(
        &self,
        perm_challenges: [SC::Challenge; 2],
        public_values: &[Val<SC>],
    ) -> Vec<BTreeSet<TraceEntry>>;

    fn track_failing_interactions(&self) -> Vec<BTreeSet<TraceEntry>>;

    fn write_traces_to_file(
        &self,
        path: &str,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    ) -> Result<(), Box<dyn Error>>;
}

#[cfg(feature = "trace-writer")]
impl<SC, C> MachineTraceDebugger<SC> for MachineTrace<SC, C>
where
    SC: StarkGenericConfig,
    Val<SC>: PrimeField32,
    C: MachineChip<SC>,
{
    fn track_failing_constraints(
        &self,
        perm_challenges: [SC::Challenge; 2],
        public_values: &[Val<SC>],
    ) -> Vec<BTreeSet<TraceEntry>> {
        let mut chip_indices = Vec::new();
        for chip_trace in self.iter() {
            let preprocessed = chip_trace
                .preprocessed
                .as_ref()
                .map(|preprocessed| preprocessed.trace.value.as_view());
            let main = chip_trace
                .main
                .as_ref()
                .map(|main| main.trace.value.as_view());
            let permutation = chip_trace
                .permutation
                .as_ref()
                .map(|permutation| permutation.trace.value.as_view());
            let indices = track_failing_constraints(
                &chip_trace.chip,
                &preprocessed,
                &main,
                &permutation,
                perm_challenges,
                chip_trace.cumulative_sum,
                public_values,
            );
            chip_indices.push(indices);
        }
        chip_indices
    }

    fn track_failing_interactions(&self) -> Vec<BTreeSet<TraceEntry>> {
        let preprocessed_traces = self
            .iter()
            .map(|chip_trace| {
                chip_trace
                    .preprocessed
                    .as_ref()
                    .map(|preprocessed| preprocessed.trace.value.as_view())
            })
            .collect_vec();
        let main_traces = self
            .iter()
            .map(|chip_trace| {
                chip_trace
                    .main
                    .as_ref()
                    .map(|main| main.trace.value.as_view())
            })
            .collect_vec();

        let airs = self
            .iter()
            .map(|chip_trace| chip_trace.chip.clone())
            .collect_vec();

        track_failing_interactions(&airs, &preprocessed_traces, &main_traces)
    }

    fn write_traces_to_file(
        &self,
        path: &str,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    ) -> Result<(), Box<dyn Error>>
    where
        Val<SC>: PrimeField32,
    {
        use rust_xlsxwriter::Workbook;

        let mut workbook = Workbook::new();

        // TODO: Account for public values
        let mut entries = vec![BTreeSet::new(); self.len()];
        self.track_failing_constraints(perm_challenges, &[])
            .iter()
            .zip(&mut entries)
            .for_each(|(entry, set)| set.extend(entry));
        self.track_failing_interactions()
            .iter()
            .zip(&mut entries)
            .for_each(|(entry, set)| set.extend(entry));

        for (chip_trace, chip_entries) in self.iter().zip(entries) {
            let chip = &chip_trace.chip;

            let worksheet = workbook.add_worksheet();
            worksheet.set_name(format!("{}", chip))?;

            let preprocessed_trace = chip_trace
                .preprocessed
                .as_ref()
                .map(|preprocessed| preprocessed.trace.value.as_view());
            let main_trace = chip_trace
                .main
                .as_ref()
                .map(|main| main.trace.value.as_view());

            chip.write_traces_to_worksheet(
                worksheet,
                &preprocessed_trace,
                &main_trace,
                chip.all_interactions(),
                chip_entries,
            )?;
        }

        workbook.save(path)?;

        Ok(())
    }
}

pub trait MachineTraceOpener<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn generate_rounds(
        &self,
        zeta: SC::Challenge,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        quotient_data: &'a Option<PcsProverData<SC>>,
    ) -> Vec<(&'a PcsProverData<SC>, Vec<Vec<SC::Challenge>>)>;

    fn unflatten_openings(
        &self,
        opening_values: Vec<OpenedValuesForRound<SC::Challenge>>,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        quotient_data: &'a Option<PcsProverData<SC>>,
    ) -> Vec<OpenedValues<SC::Challenge>>;

    fn generate_proofs(
        &self,
        openings: Vec<OpenedValues<SC::Challenge>>,
    ) -> Vec<Option<InteractionAirProof<SC::Challenge>>>;
}

impl<'a, SC, C> MachineTraceOpener<'a, SC> for MachineTrace<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn generate_rounds(
        &self,
        zeta: SC::Challenge,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        quotient_data: &'a Option<PcsProverData<SC>>,
    ) -> Vec<(&'a PcsProverData<SC>, Vec<Vec<SC::Challenge>>)> {
        let mut rounds = vec![];
        if let Some(preprocessed_data) = preprocessed_data {
            let opening_points = self
                .iter()
                .flat_map(|chip_trace| {
                    chip_trace.preprocessed.as_ref().map(|preprocessed| {
                        let domain = preprocessed.trace.domain;
                        vec![zeta, domain.next_point(zeta).unwrap()]
                    })
                })
                .collect_vec();
            rounds.push((preprocessed_data, opening_points));
        }
        if let Some(main_data) = main_data {
            let opening_points = self
                .iter()
                .flat_map(|chip_trace| {
                    chip_trace.main.as_ref().map(|main| {
                        let domain = main.trace.domain;
                        vec![zeta, domain.next_point(zeta).unwrap()]
                    })
                })
                .collect_vec();
            rounds.push((main_data, opening_points));
        }
        if let Some(permutation_data) = permutation_data {
            let opening_points = self
                .iter()
                .flat_map(|chip_trace| {
                    chip_trace.permutation.as_ref().map(|permutation| {
                        let domain = permutation.trace.domain;
                        vec![zeta, domain.next_point(zeta).unwrap()]
                    })
                })
                .collect_vec();
            rounds.push((permutation_data, opening_points));
        }
        if let Some(quotient_data) = quotient_data {
            // open every chunk at zeta
            let opening_points = self
                .iter()
                .flat_map(|chip_trace| {
                    chip_trace
                        .quotient_degree
                        .map(|degree| (0..degree).map(|_| vec![zeta]).collect_vec())
                })
                .flatten()
                .collect_vec();
            rounds.push((quotient_data, opening_points));
        }

        rounds
    }

    fn unflatten_openings(
        &self,
        mut opening_values: Vec<OpenedValuesForRound<SC::Challenge>>,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        quotient_data: &'a Option<PcsProverData<SC>>,
    ) -> Vec<OpenedValues<SC::Challenge>> {
        let quotient_openings = if quotient_data.is_some() {
            // Unflatten quotient openings
            let openings = opening_values.pop().expect("Opening should be present");
            let mut start = 0;
            // TODO: use drain
            // TODO: remove clone
            self.iter()
                .map(|chip_proof| {
                    chip_proof.quotient_degree.map(|degree| {
                        let end = start + degree;
                        let openings = openings[start..end]
                            .iter()
                            .map(|chunk| {
                                assert_eq!(chunk.len(), 1, "Should have 1 opening");
                                chunk[0].clone()
                            })
                            .collect_vec();
                        start = end;
                        openings
                    })
                })
                .collect_vec()
        } else {
            // TODO: Better way
            self.iter().map(|_| None).collect_vec()
        };

        let permutation_openings = if permutation_data.is_some() {
            let openings = opening_values.pop().expect("Opening should be present");
            // TODO: remove clone
            self.iter()
                .map(|chip_trace| {
                    chip_trace.permutation.as_ref().map(|permutation| {
                        let openings = &openings[permutation.opening_index];
                        assert_eq!(openings.len(), 2, "Should have 2 openings");
                        AdjacentOpenedValues {
                            local: openings[0].clone(),
                            next: openings[1].clone(),
                        }
                    })
                })
                .collect_vec()
        } else {
            // TODO: Better way
            self.iter().map(|_| None).collect_vec()
        };

        let main_openings = if main_data.is_some() {
            let openings = opening_values.pop().expect("Opening should be present");
            self.iter()
                .map(|chip_trace| {
                    chip_trace.main.as_ref().map(|main| {
                        let openings = &openings[main.opening_index];
                        assert_eq!(openings.len(), 2, "Should have 2 openings");
                        AdjacentOpenedValues {
                            local: openings[0].clone(),
                            next: openings[1].clone(),
                        }
                    })
                })
                .collect_vec()
        } else {
            // TODO: Better way
            self.iter().map(|_| None).collect_vec()
        };

        let preprocessed_openings = if preprocessed_data.is_some() {
            let openings = opening_values.pop().expect("Opening should be present");
            self.iter()
                .map(|chip_trace| {
                    chip_trace.preprocessed.as_ref().map(|preprocessed| {
                        let openings = &openings[preprocessed.opening_index];
                        assert_eq!(openings.len(), 2, "Should have 2 openings");
                        AdjacentOpenedValues {
                            local: openings[0].clone(),
                            next: openings[1].clone(),
                        }
                    })
                })
                .collect_vec()
        } else {
            // TODO: Better way
            self.iter().map(|_| None).collect_vec()
        };

        preprocessed_openings
            .into_iter()
            .zip_eq(main_openings)
            .zip_eq(permutation_openings)
            .zip_eq(quotient_openings)
            .map(
                |(((preprocessed, main), permutation), quotient_chunks)| OpenedValues {
                    preprocessed,
                    main,
                    permutation,
                    quotient_chunks,
                },
            )
            .collect()
    }

    fn generate_proofs(
        &self,
        openings: Vec<OpenedValues<SC::Challenge>>,
    ) -> Vec<Option<InteractionAirProof<SC::Challenge>>> {
        self.iter()
            .zip_eq(openings)
            .map(|(chip_trace, opened_values)| {
                chip_trace.domain().map(|domain| {
                    let degree = domain.size();
                    let cumulative_sum = chip_trace.cumulative_sum;

                    InteractionAirProof {
                        degree,
                        opened_values,
                        cumulative_sum,
                    }
                })
            })
            .collect()
    }
}

fn load_traces<SC, F>(
    pcs: &SC::Pcs,
    traces: Vec<Option<RowMajorMatrix<F>>>,
) -> Vec<Option<IndexedTrace<F, Domain<SC>>>>
where
    F: Field,
    SC: StarkGenericConfig,
{
    let mut count = 0;
    traces
        .into_iter()
        .map(|mt| {
            if let Some(trace) = mt {
                let degree = trace.height();
                if degree > 0 {
                    let domain = pcs.natural_domain_for_degree(degree);
                    let trace = Trace {
                        value: trace,
                        domain,
                    };
                    let index = count;
                    count += 1;

                    Some(IndexedTrace {
                        trace,
                        opening_index: index,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn commit_traces<SC>(
    pcs: &SC::Pcs,
    traces: Vec<Trace<Val<SC>, Domain<SC>>>,
) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
where
    SC: StarkGenericConfig,
{
    let domains_and_traces: Vec<_> = traces
        .into_iter()
        .map(|trace| (trace.domain, trace.value))
        .collect();
    if !domains_and_traces.is_empty() {
        let (commit, data) = pcs.commit(domains_and_traces);
        (Some(commit), Some(data))
    } else {
        (None, None)
    }
}

#[derive(Clone)]
pub struct TraceOpening<EF, Domain>
where
    EF: Field,
    Domain: PolynomialSpace,
{
    pub values: AdjacentOpenedValues<EF>,
    pub domain: Domain,
}

#[derive(Clone)]
pub struct SingleQuotientTraceOpening<EF, Domain>
where
    EF: Field,
    Domain: PolynomialSpace,
{
    pub values: Vec<EF>,
    pub domain: Domain,
}

#[derive(Clone)]
pub struct QuotientTraceOpening<EF, Domain>
where
    EF: Field,
    Domain: PolynomialSpace,
{
    pub traces: Vec<SingleQuotientTraceOpening<EF, Domain>>,
    // pub opening_index: usize,
}

#[derive(Clone)]
pub struct ChipTraceOpening<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    pub chip: C,

    pub preprocessed: Option<TraceOpening<SC::Challenge, Domain<SC>>>,
    pub main: Option<TraceOpening<SC::Challenge, Domain<SC>>>,
    pub permutation: Option<TraceOpening<SC::Challenge, Domain<SC>>>,

    pub cumulative_sum: Option<SC::Challenge>,

    pub quotient_chunks: Option<QuotientTraceOpening<SC::Challenge, Domain<SC>>>,
    pub quotient_degree: Option<usize>,
}

impl<SC, C> ChipTraceOpening<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    pub fn new(chip: C) -> Self {
        Self {
            chip,
            preprocessed: None,
            main: None,
            permutation: None,
            cumulative_sum: None,
            quotient_chunks: None,
            quotient_degree: None,
        }
    }

    // TODO: Change to be just main degree
    pub fn domain(&self) -> Option<Domain<SC>> {
        match (&self.preprocessed, &self.main) {
            (Some(preprocessed), Some(main)) => {
                let preprocessed_domain = preprocessed.domain;
                let main_domain = main.domain;
                if main_domain.size() > preprocessed_domain.size() {
                    Some(main_domain)
                } else {
                    Some(preprocessed_domain)
                }
            }
            (Some(preprocessed), None) => Some(preprocessed.domain),
            (None, Some(main)) => Some(main.domain),
            (None, None) => None,
        }
    }
}

pub type MachineTraceOpening<SC, C> = Vec<ChipTraceOpening<SC, C>>;

pub trait MachineTraceOpeningBuilder<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn new(chips: &[C]) -> Self;
}

impl<SC, C> MachineTraceOpeningBuilder<SC, C> for MachineTraceOpening<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn new(chips: &[C]) -> Self {
        chips
            .iter()
            .map(|chip| ChipTraceOpening::new(chip.clone()))
            .collect_vec()
    }
}

pub trait MachineTraceOpeningLoader<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn load_openings(
        &mut self,
        pcs: &'a SC::Pcs,
        chip_proofs: Vec<Option<InteractionAirProof<SC::Challenge>>>,
        preprocessed_degrees: Vec<usize>,
    );

    fn verify_shapes(&self) -> Result<(), VerificationError>;
}

impl<'a, SC, C> MachineTraceOpeningLoader<'a, SC> for Vec<ChipTraceOpening<SC, C>>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn load_openings(
        &mut self,
        pcs: &'a SC::Pcs,
        chip_proofs: Vec<Option<InteractionAirProof<SC::Challenge>>>,
        preprocessed_degrees: Vec<usize>,
    ) {
        for ((chip_trace, chip_proof), preprocessed_degree) in self
            .iter_mut()
            .zip_eq(chip_proofs.into_iter())
            .zip_eq(preprocessed_degrees.into_iter())
        {
            if let Some(proof) = chip_proof {
                chip_trace.preprocessed = proof.opened_values.preprocessed.map(|values| {
                    let domain = pcs.natural_domain_for_degree(preprocessed_degree);
                    TraceOpening { values, domain }
                });

                let domain = pcs.natural_domain_for_degree(proof.degree);
                chip_trace.main = proof
                    .opened_values
                    .main
                    .map(|values| TraceOpening { values, domain });
                chip_trace.permutation = proof
                    .opened_values
                    .permutation
                    .map(|values| TraceOpening { values, domain });
                chip_trace.cumulative_sum = proof.cumulative_sum;

                // TODO: Pub values
                let quotient_degree = get_quotient_degree::<Val<SC>, _>(&chip_trace.chip, 0);
                chip_trace.quotient_degree = Some(quotient_degree);

                let quotient_domain =
                    domain.create_disjoint_domain(domain.size() * quotient_degree);
                let quotient_chunks_domains = quotient_domain.split_domains(quotient_degree);
                chip_trace.quotient_chunks = proof.opened_values.quotient_chunks.map(|chunks| {
                    let values = chunks
                        .into_iter()
                        .zip_eq(quotient_chunks_domains.into_iter())
                        .map(|(chunk, domain)| SingleQuotientTraceOpening {
                            values: chunk,
                            domain,
                        })
                        .collect();
                    QuotientTraceOpening { traces: values }
                });
            }
        }
    }

    fn verify_shapes(&self) -> Result<(), VerificationError> {
        // TODO: Add preprocessed and permutation size check
        for chip_trace in self.iter() {
            // TODO: Try to do without the cast
            let main_width = <C as BaseAir<Val<SC>>>::width(&chip_trace.chip);

            if let Some(main) = &chip_trace.main {
                if main.values.local.len() != main_width {
                    return Err(VerificationError::InvalidProofShape);
                }
                if main.values.next.len() != main_width {
                    return Err(VerificationError::InvalidProofShape);
                }
            }
            if let Some(quotient_chunks) = &chip_trace.quotient_chunks {
                // TODO: Pub values
                let quotient_degree = get_quotient_degree::<Val<SC>, _>(&chip_trace.chip, 0);
                if quotient_chunks.traces.len() != quotient_degree {
                    return Err(VerificationError::InvalidProofShape);
                }
                if !quotient_chunks.traces.iter().all(|qc| {
                    qc.values.len() == <SC::Challenge as AbstractExtensionField<Val<SC>>>::D
                }) {
                    return Err(VerificationError::InvalidProofShape);
                }
            }
        }

        Ok(())
    }
}

pub trait MachineTraceOpeningVerifier<SC>
where
    SC: StarkGenericConfig,
{
    fn generate_rounds(
        &self,
        zeta: SC::Challenge,
        preprocessed_commitment: &Option<Com<SC>>,
        main_commitment: &Option<Com<SC>>,
        permutation_commitment: &Option<Com<SC>>,
        quotient_chunks_commitment: &Option<Com<SC>>,
    ) -> Vec<(
        Com<SC>,
        Vec<(Domain<SC>, Vec<(SC::Challenge, Vec<SC::Challenge>)>)>,
    )>;
}

impl<SC, C> MachineTraceOpeningVerifier<SC> for MachineTraceOpening<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn generate_rounds(
        &self,
        zeta: SC::Challenge,
        preprocessed_commitment: &Option<Com<SC>>,
        main_commitment: &Option<Com<SC>>,
        permutation_commitment: &Option<Com<SC>>,
        quotient_chunks_commitment: &Option<Com<SC>>,
    ) -> Vec<(
        Com<SC>,
        Vec<(Domain<SC>, Vec<(SC::Challenge, Vec<SC::Challenge>)>)>,
    )> {
        let mut rounds = vec![];
        if let Some(preprocessed_commitment) = preprocessed_commitment {
            let preprocessed_domains_and_openings = self
                .iter()
                .filter_map(|chip_trace| {
                    chip_trace.preprocessed.as_ref().map(|trace| {
                        (
                            trace.domain,
                            vec![
                                (zeta, trace.values.local.clone()),
                                (
                                    trace.domain.next_point(zeta).unwrap(),
                                    trace.values.next.clone(),
                                ),
                            ],
                        )
                    })
                })
                .collect_vec();
            rounds.push((
                preprocessed_commitment.clone(),
                preprocessed_domains_and_openings,
            ));
        }
        if let Some(main_commitment) = main_commitment {
            let main_domains_and_openings = self
                .iter()
                .filter_map(|chip_trace| {
                    chip_trace.main.as_ref().map(|trace| {
                        (
                            trace.domain,
                            vec![
                                (zeta, trace.values.local.clone()),
                                (
                                    trace.domain.next_point(zeta).unwrap(),
                                    trace.values.next.clone(),
                                ),
                            ],
                        )
                    })
                })
                .collect_vec();
            rounds.push((main_commitment.clone(), main_domains_and_openings));
        }
        if let Some(permutation_commitment) = permutation_commitment {
            let permutation_domains_and_openings = self
                .iter()
                .filter_map(|chip_trace| {
                    chip_trace.permutation.as_ref().map(|trace| {
                        (
                            trace.domain,
                            vec![
                                (zeta, trace.values.local.clone()),
                                (
                                    trace.domain.next_point(zeta).unwrap(),
                                    trace.values.next.clone(),
                                ),
                            ],
                        )
                    })
                })
                .collect_vec();
            rounds.push((
                permutation_commitment.clone(),
                permutation_domains_and_openings,
            ));
        }
        if let Some(quotient_chunks_commitment) = quotient_chunks_commitment {
            let quotient_chunks_domains_and_openings = self
                .iter()
                .filter_map(|chip_trace| {
                    chip_trace.quotient_chunks.as_ref().map(|quotient_trace| {
                        quotient_trace
                            .traces
                            .iter()
                            .map(|trace| (trace.domain, vec![(zeta, trace.values.clone())]))
                            .collect_vec()
                    })
                })
                .flatten()
                .collect_vec();
            rounds.push((
                quotient_chunks_commitment.clone(),
                quotient_chunks_domains_and_openings,
            ));
        }
        rounds
    }
}

pub trait MachineTraceConstraintVerifier<SC>
where
    SC: StarkGenericConfig,
{
    fn verify_constraints(
        &self,
        zeta: SC::Challenge,
        alpha: SC::Challenge,
        permutation_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        public_values: &[Val<SC>],
    ) -> Result<(), VerificationError>;

    fn verify_cumulative_sums(&self) -> Result<(), VerificationError>;
}

impl<SC, C> MachineTraceConstraintVerifier<SC> for MachineTraceOpening<SC, C>
where
    SC: StarkGenericConfig,
    C: MachineChip<SC>,
{
    fn verify_constraints(
        &self,
        zeta: SC::Challenge,
        alpha: SC::Challenge,
        permutation_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        public_values: &[Val<SC>],
    ) -> Result<(), VerificationError> {
        for chip_trace in self.iter() {
            if let Some(domain) = chip_trace.domain() {
                let qc_domains = chip_trace
                    .quotient_chunks
                    .as_ref()
                    .expect("Quotient chunks should be present")
                    .traces
                    .iter()
                    .map(|trace| trace.domain)
                    .collect_vec();
                // TODO: Remove clones
                let opened_values = OpenedValues {
                    preprocessed: chip_trace
                        .preprocessed
                        .as_ref()
                        .map(|trace| trace.values.clone()),
                    main: chip_trace.main.as_ref().map(|trace| trace.values.clone()),
                    permutation: chip_trace
                        .permutation
                        .as_ref()
                        .map(|trace| trace.values.clone()),
                    quotient_chunks: chip_trace.quotient_chunks.as_ref().map(|chunk| {
                        chunk
                            .traces
                            .iter()
                            .map(|trace| trace.values.clone())
                            .collect_vec()
                    }),
                };
                verify_constraints::<SC, _>(
                    &chip_trace.chip,
                    &opened_values,
                    domain,
                    &qc_domains,
                    zeta,
                    alpha,
                    permutation_challenges,
                    chip_trace.cumulative_sum,
                    public_values,
                )?;
            }
        }
        Ok(())
    }

    fn verify_cumulative_sums(&self) -> Result<(), VerificationError> {
        let sum: SC::Challenge = self
            .iter()
            .flat_map(|chip_trace| chip_trace.cumulative_sum)
            .sum();

        if sum != SC::Challenge::zero() {
            return Err(VerificationError::NonZeroCumulativeSum);
        }
        Ok(())
    }
}

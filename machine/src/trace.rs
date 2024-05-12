use itertools::Itertools;
use p3_air::BaseAir;
use p3_commit::{OpenedValuesForRound, Pcs, PolynomialSpace};
use p3_field::{AbstractField, ExtensionField, Field};
use p3_interaction::{
    generate_permutation_trace, InteractionAir, InteractionAirBuilder, NUM_PERM_CHALLENGES,
};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_stark::{symbolic::get_quotient_degree, AdjacentOpenedValues, ChipProof, OpenedValues};
use p3_uni_stark::{Com, Domain, PackedChallenge, StarkGenericConfig, Val};

use crate::{chip::ChipType, proof::PcsProverData, quotient::quotient_values};

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
pub struct ChipTrace<'a, SC>
where
    SC: StarkGenericConfig,
{
    pub chip: &'a ChipType,

    pub preprocessed: Option<IndexedTrace<Val<SC>, Domain<SC>>>,
    pub main: Option<IndexedTrace<Val<SC>, Domain<SC>>>,
    pub permutation: Option<IndexedTrace<SC::Challenge, Domain<SC>>>,

    pub cumulative_sum: Option<SC::Challenge>,

    pub quotient_chunks: Option<QuotientTrace<Domain<SC>>>,
    pub quotient_degree: Option<usize>,
}

impl<'a, SC> ChipTrace<'a, SC>
where
    SC: StarkGenericConfig,
{
    pub fn new(chip: &'a ChipType) -> Self {
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

pub type MachineTrace<'a, SC> = Vec<ChipTrace<'a, SC>>;

pub trait MachineTraceBuilder<'a> {
    fn new(chips: Vec<&'a ChipType>) -> Self;
}

impl<'a, SC> MachineTraceBuilder<'a> for MachineTrace<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn new(chips: Vec<&'a ChipType>) -> Self {
        chips
            .into_iter()
            .map(|chip| ChipTrace::new(chip))
            .collect_vec()
    }
}

pub trait MachineTraceLoader<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn generate_preprocessed(self, pcs: &'a SC::Pcs) -> Self;

    fn load_preprocessed(
        self,
        pcs: &'a SC::Pcs,
        traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
    ) -> Self;

    fn load_main(self, pcs: &'a SC::Pcs, traces: Vec<Option<RowMajorMatrix<Val<SC>>>>) -> Self;

    fn generate_permutation<AB>(
        self,
        pcs: &'a SC::Pcs,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    ) -> Self
    where
        AB: InteractionAirBuilder<Expr = Val<SC>>;

    fn generate_quotient(
        self,
        pcs: &'a SC::Pcs,
        preprocessed_data: Option<PcsProverData<SC>>,
        main_data: Option<PcsProverData<SC>>,
        permutation_data: Option<PcsProverData<SC>>,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        alpha: SC::Challenge,
    ) -> Self;
}

impl<'a, SC> MachineTraceLoader<'a, SC> for MachineTrace<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn generate_preprocessed(mut self, pcs: &'a SC::Pcs) -> Self {
        let traces = self
            .iter()
            .map(|trace| trace.chip.preprocessed_trace())
            .collect_vec();
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, preprocessed) in self.iter_mut().zip_eq(traces) {
            chip_trace.preprocessed = preprocessed;
        }
        self
    }

    fn load_preprocessed(
        mut self,
        pcs: &'a SC::Pcs,
        traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
    ) -> Self {
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, preprocessed) in self.iter_mut().zip_eq(traces) {
            chip_trace.preprocessed = preprocessed;
        }
        self
    }

    fn load_main(mut self, pcs: &'a SC::Pcs, traces: Vec<Option<RowMajorMatrix<Val<SC>>>>) -> Self {
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, main) in self.iter_mut().zip_eq(traces) {
            chip_trace.main = main;
        }
        self
    }

    fn generate_permutation<AB>(
        mut self,
        pcs: &'a SC::Pcs,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    ) -> Self
    where
        AB: InteractionAirBuilder<Expr = Val<SC>>,
    {
        let traces = self
            .iter()
            .map(|trace| {
                let preprocessed = trace
                    .preprocessed
                    .as_ref()
                    .map(|mt| mt.trace.value.as_view());
                let main = trace.main.as_ref().map(|mt| mt.trace.value.as_view());
                let interactions = <ChipType as InteractionAir<AB>>::all_interactions(trace.chip);

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
        self
    }

    fn generate_quotient(
        mut self,
        pcs: &'a SC::Pcs,
        preprocessed_data: Option<PcsProverData<SC>>,
        main_data: Option<PcsProverData<SC>>,
        permutation_data: Option<PcsProverData<SC>>,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        alpha: SC::Challenge,
    ) -> Self {
        let perm_challenges = perm_challenges.map(PackedChallenge::<SC>::from_f);
        let alpha = PackedChallenge::<SC>::from_f(alpha);

        let mut count = 0;
        for chip_trace in self.iter_mut() {
            let quotient_degree = get_quotient_degree::<Val<SC>, _>(chip_trace.chip, 0);
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
                        RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                    };
                let main_trace_on_quotient_domains = if let Some(main) = &chip_trace.main {
                    pcs.get_evaluations_on_domain(
                        main_data.as_ref().unwrap(),
                        main.opening_index,
                        quotient_domain,
                    )
                    .to_row_major_matrix()
                } else {
                    RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
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
                        RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                    };

                let cumulative_sum = chip_trace
                    .cumulative_sum
                    .map(PackedChallenge::<SC>::from_f)
                    .unwrap_or_default();

                let quotient_values = quotient_values::<SC, _, _>(
                    chip_trace.chip,
                    trace_domain,
                    quotient_domain,
                    preprocessed_trace_on_quotient_domains,
                    main_trace_on_quotient_domains,
                    perm_trace_on_quotient_domains,
                    perm_challenges,
                    alpha,
                    cumulative_sum,
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

                chip_trace.quotient_chunks = Some(QuotientTrace {
                    traces,
                    opening_index: count,
                });
                count += 1;
            }
        }

        self
    }
}

pub trait MachineTraceCommiter<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn commit_preprocessed(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>);

    fn commit_main(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>);

    fn commit_permutation(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>);

    fn commit_quotient(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>);
}

impl<'a, SC> MachineTraceCommiter<'a, SC> for MachineTrace<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn commit_preprocessed(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .into_iter()
            .flat_map(|trace| trace.preprocessed.map(|preprocessed| preprocessed.trace))
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_main(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .into_iter()
            .flat_map(|trace| trace.main.map(|main| main.trace))
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_permutation(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .into_iter()
            .flat_map(|trace| {
                trace
                    .permutation
                    .map(|permutation| permutation.trace.flatten_to_base())
            })
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_quotient(self, pcs: &'a SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>) {
        let traces = self
            .into_iter()
            .flat_map(|trace| trace.quotient_chunks.map(|quotient| quotient.traces))
            .flatten()
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }
}

pub trait MachineTraceOpener<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn generate_rounds(
        self,
        zeta: SC::Challenge,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        quotient_data: &'a Option<PcsProverData<SC>>,
    ) -> Vec<(&'a PcsProverData<SC>, Vec<Vec<SC::Challenge>>)>;

    fn unflatten_openings(
        self,
        opening_values: Vec<OpenedValuesForRound<SC::Challenge>>,
        preprocessed_data: &'a Option<PcsProverData<SC>>,
        main_data: &'a Option<PcsProverData<SC>>,
        permutation_data: &'a Option<PcsProverData<SC>>,
        quotient_data: &'a Option<PcsProverData<SC>>,
    ) -> Vec<OpenedValues<SC::Challenge>>;

    fn generate_proofs(
        self,
        openings: Vec<OpenedValues<SC::Challenge>>,
    ) -> Vec<Option<ChipProof<SC::Challenge>>>;
}

impl<'a, SC> MachineTraceOpener<'a, SC> for MachineTrace<'a, SC>
where
    SC: StarkGenericConfig,
{
    fn generate_rounds(
        self,
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
        self,
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
        self,
        openings: Vec<OpenedValues<SC::Challenge>>,
    ) -> Vec<Option<ChipProof<SC::Challenge>>> {
        self.iter()
            .zip_eq(openings)
            .map(|(chip_trace, opened_values)| {
                chip_trace.domain().map(|domain| {
                    let degree = domain.size();
                    let cumulative_sum = chip_trace.cumulative_sum;

                    ChipProof {
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

use itertools::Itertools;
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{AbstractField, ExtensionField, Field};
use p3_interaction::{
    generate_permutation_trace, InteractionAir, InteractionAirBuilder, NUM_PERM_CHALLENGES,
};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_stark::symbolic::get_quotient_degree;
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
    pub opening_index: usize,
}

#[derive(Clone)]
pub struct QuotientTrace<Domain>
where
    Domain: PolynomialSpace,
{
    pub values: Vec<RowMajorMatrix<Domain::Val>>,
    pub domains: Vec<Domain>,
    pub opening_index: usize,
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
            opening_index: self.opening_index,
        }
    }
}

#[derive(Clone)]
pub struct ChipTrace<'a, Domain, EF>
where
    Domain: PolynomialSpace,
    EF: ExtensionField<Domain::Val>,
{
    pub chip: &'a ChipType,

    pub preprocessed: Option<Trace<Domain::Val, Domain>>,
    pub main: Option<Trace<Domain::Val, Domain>>,
    pub permutation: Option<Trace<EF, Domain>>,

    pub cumulative_sum: Option<EF>,

    pub quotient_chunks: Option<QuotientTrace<Domain>>,
    pub quotient_degree: Option<usize>,
}

impl<'a, Domain, EF> ChipTrace<'a, Domain, EF>
where
    Domain: PolynomialSpace,
    EF: ExtensionField<Domain::Val>,
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

    pub fn domain(&self) -> Option<Domain> {
        match (&self.preprocessed, &self.main) {
            (Some(preprocessed_trace), Some(main_trace)) => {
                let preprocessed_domain = preprocessed_trace.domain;
                let main_domain = main_trace.domain;
                if main_domain.size() > preprocessed_domain.size() {
                    Some(main_domain)
                } else {
                    Some(preprocessed_domain)
                }
            }
            (Some(preprocessed_trace), None) => Some(preprocessed_trace.domain),
            (None, Some(main_trace)) => Some(main_trace.domain),
            (None, None) => None,
        }
    }
}

pub type MachineTrace<'a, Domain, EF> = Vec<ChipTrace<'a, Domain, EF>>;

pub trait MachineTraceBuilder<'a> {
    fn new(chips: &'a [&ChipType]) -> Self;
}

impl<'a, Domain, EF> MachineTraceBuilder<'a> for MachineTrace<'a, Domain, EF>
where
    Domain: PolynomialSpace,
    EF: ExtensionField<Domain::Val>,
{
    fn new(chips: &'a [&ChipType]) -> Self {
        chips.iter().map(|chip| ChipTrace::new(chip)).collect_vec()
    }
}

pub trait MachineTraceLoader<'a, Domain, SC>
where
    Domain: PolynomialSpace,
    SC: StarkGenericConfig,
    SC::Challenge: ExtensionField<Domain::Val>,
{
    fn load_preprocessed<P>(
        self,
        pcs: &P,
        traces: Vec<Option<RowMajorMatrix<Domain::Val>>>,
    ) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>;

    fn load_main<P>(self, pcs: &P, traces: Vec<Option<RowMajorMatrix<Domain::Val>>>) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>;

    fn generate_permutation<P, AB>(
        self,
        pcs: &P,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    ) -> Self
    where
        AB: InteractionAirBuilder<Expr = Domain::Val>,
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>;

    fn generate_quotient<P>(
        self,
        pcs: &P,
        preprocessed_data: Option<PcsProverData<SC>>,
        main_data: Option<PcsProverData<SC>>,
        permutation_data: Option<PcsProverData<SC>>,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        alpha: SC::Challenge,
    ) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>;
}

impl<'a, Domain, SC> MachineTraceLoader<'a, Domain, SC> for MachineTrace<'a, Domain, SC::Challenge>
where
    Domain: PolynomialSpace,
    SC: StarkGenericConfig,
    SC::Challenge: ExtensionField<Domain::Val>,
{
    fn load_preprocessed<P>(
        mut self,
        pcs: &P,
        traces: Vec<Option<RowMajorMatrix<Domain::Val>>>,
    ) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
    {
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, preprocessed) in self.iter_mut().zip_eq(traces) {
            chip_trace.preprocessed = preprocessed;
        }
        self
    }

    fn load_main<P>(mut self, pcs: &P, traces: Vec<Option<RowMajorMatrix<Domain::Val>>>) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
    {
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, main) in self.iter_mut().zip_eq(traces) {
            chip_trace.main = main;
        }
        self
    }

    fn generate_permutation<P, AB>(
        mut self,
        pcs: &P,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
    ) -> Self
    where
        AB: InteractionAirBuilder<Expr = Domain::Val>,
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
    {
        let traces = self
            .iter()
            .map(|trace| {
                let value = generate_permutation_trace(
                    &trace.preprocessed.as_ref().map(|mt| mt.value.as_view()),
                    &trace.main.as_ref().map(|mt| mt.value.as_view()),
                    <ChipType as InteractionAir<AB>>::all_interactions(trace.chip).as_slice(),
                    perm_challenges,
                );
                value
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

    fn generate_quotient<P>(
        mut self,
        pcs: &P,
        preprocessed_data: Option<PcsProverData<SC>>,
        main_data: Option<PcsProverData<SC>>,
        permutation_data: Option<PcsProverData<SC>>,
        perm_challenges: [SC::Challenge; NUM_PERM_CHALLENGES],
        alpha: SC::Challenge,
    ) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
    {
        let perm_challenges = perm_challenges.map(PackedChallenge::<SC>::from_f);
        let alpha = PackedChallenge::<SC>::from_f(alpha);

        let mut count = 0;
        for trace in self.iter_mut() {
            let quotient_degree = get_quotient_degree::<Val<SC>, _>(trace.chip, 0);
            let trace_domain = trace.domain();

            if let Some(trace_domain) = trace_domain {
                let quotient_domain =
                    trace_domain.create_disjoint_domain(trace_domain.size() * quotient_degree);

                let preprocessed_trace_on_quotient_domains =
                    if let Some(preprocessed) = &trace.preprocessed {
                        pcs.get_evaluations_on_domain(
                            preprocessed_data.as_ref().unwrap(),
                            preprocessed.opening_index,
                            quotient_domain,
                        )
                        .to_row_major_matrix()
                    } else {
                        RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                    };
                let main_trace_on_quotient_domains = if let Some(main) = &trace.main {
                    pcs.get_evaluations_on_domain(
                        main_data.as_ref().unwrap(),
                        main.opening_index,
                        quotient_domain,
                    )
                    .to_row_major_matrix()
                } else {
                    RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                };
                let perm_trace_on_quotient_domains = if let Some(permutation) = &trace.permutation {
                    pcs.get_evaluations_on_domain(
                        permutation_data.as_ref().unwrap(),
                        permutation.opening_index,
                        quotient_domain,
                    )
                    .to_row_major_matrix()
                } else {
                    RowMajorMatrix::new_col(vec![Val::<SC>::zero(); quotient_domain.size()])
                };

                let cumulative_sum = trace
                    .cumulative_sum
                    .map(PackedChallenge::<SC>::from_f)
                    .unwrap_or_default();

                let quotient_values = quotient_values::<SC, _, _>(
                    trace.chip,
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

                let quotient_chunks = quotient_domain.split_evals(quotient_degree, quotient_flat);
                let chunk_domains = quotient_domain.split_domains(quotient_degree);

                trace.quotient_chunks = Some(QuotientTrace {
                    values: quotient_chunks,
                    domains: chunk_domains,
                    opening_index: count,
                });
                count += 1;
            }
        }

        self
    }
}

pub trait MachineTraceCommiter<'a, Domain, EF>
where
    Domain: PolynomialSpace,
    EF: ExtensionField<Domain::Val>,
{
    fn commit_preprocessed<SC>(self, pcs: &SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        SC: StarkGenericConfig,
        SC::Pcs: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>;

    fn commit_main<SC>(self, pcs: &SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        SC: StarkGenericConfig,
        SC::Pcs: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>;

    fn commit_permutation<SC>(self, pcs: &SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        SC: StarkGenericConfig,
        SC::Pcs: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>;
}

impl<'a, Domain, EF> MachineTraceCommiter<'a, Domain, EF> for MachineTrace<'a, Domain, EF>
where
    Domain: PolynomialSpace,
    EF: ExtensionField<Domain::Val>,
{
    fn commit_preprocessed<SC>(self, pcs: &SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        SC: StarkGenericConfig,
        SC::Pcs: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
    {
        let traces = self
            .into_iter()
            .map(|trace| trace.preprocessed)
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_main<SC>(self, pcs: &SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        SC: StarkGenericConfig,
        SC::Pcs: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
    {
        let traces = self.into_iter().map(|trace| trace.main).collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_permutation<SC>(self, pcs: &SC::Pcs) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        SC: StarkGenericConfig,
        SC::Pcs: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
    {
        let traces = self
            .into_iter()
            .map(|trace| trace.permutation.map(|trace| trace.flatten_to_base()))
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }
}

fn load_traces<SC, F>(
    pcs: &SC::Pcs,
    traces: Vec<Option<RowMajorMatrix<F>>>,
) -> Vec<Option<Trace<F, Domain<SC>>>>
where
    F: Field,
    SC: StarkGenericConfig,
{
    traces
        .into_iter()
        .scan(0usize, |count, mt| {
            Some({
                if let Some(trace) = mt {
                    let degree = trace.height();
                    if degree > 0 {
                        let domain = pcs.natural_domain_for_degree(degree);
                        let index = *count;
                        *count += 1;

                        Some(Trace {
                            value: trace,
                            domain,
                            opening_index: index,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        })
        .collect()
}

fn commit_traces<SC>(
    pcs: &SC::Pcs,
    traces: Vec<Option<Trace<Val<SC>, Domain<SC>>>>,
) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
where
    SC: StarkGenericConfig,
{
    let domains_and_traces: Vec<_> = traces
        .into_iter()
        .flat_map(|mt| mt.map(|trace| (trace.domain, trace.value)))
        .collect();
    if !domains_and_traces.is_empty() {
        let (commit, data) = pcs.commit(domains_and_traces);
        (Some(commit), Some(data))
    } else {
        (None, None)
    }
}

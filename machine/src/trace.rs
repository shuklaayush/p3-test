use itertools::Itertools;
use p3_commit::{Pcs, PolynomialSpace};
use p3_field::{ExtensionField, Field};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_uni_stark::{Com, Domain, StarkGenericConfig, Val};

use crate::{chip::ChipType, proof::PcsProverData};

#[derive(Clone)]
pub struct Trace<F, Domain>
where
    F: Field,
    Domain: PolynomialSpace,
{
    pub matrix: RowMajorMatrix<F>,
    pub domain: Domain,
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
            matrix: self.matrix.flatten_to_base(),
            domain: self.domain,
            opening_index: self.opening_index,
        }
    }
}

#[derive(Clone)]
pub struct ChipTrace<'a, F, EF, Domain>
where
    F: Field,
    EF: ExtensionField<F>,
    Domain: PolynomialSpace,
{
    pub chip: &'a ChipType,

    pub preprocessed: Option<Trace<F, Domain>>,
    pub main: Option<Trace<F, Domain>>,
    pub permutation: Option<Trace<EF, Domain>>,

    pub quotient_chunks: Option<Trace<EF, Domain>>,
}

impl<'a, F, EF, Domain> ChipTrace<'a, F, EF, Domain>
where
    F: Field,
    EF: ExtensionField<F>,
    Domain: PolynomialSpace,
{
    pub fn new(chip: &'a ChipType) -> Self {
        Self {
            chip,
            preprocessed: None,
            main: None,
            permutation: None,
            quotient_chunks: None,
        }
    }

    // // 3. Calculate trace domains = max(preprocessed, main)
    // pub fn domain(&self) -> Domain {
    //     let trace_domains = pk
    //         .preprocessed_traces
    //         .iter()
    //         .zip_eq(main_traces.iter())
    //         .map(|traces| match traces {
    //             (Some(preprocessed_trace), Some(main_trace)) => {
    //                 let preprocessed_domain = preprocessed_trace.domain;
    //                 let main_domain = main_trace.domain;
    //                 if main_domain.size() > preprocessed_domain.size() {
    //                     Some(main_domain)
    //                 } else {
    //                     Some(preprocessed_domain)
    //                 }
    //             }
    //             (Some(preprocessed_trace), None) => Some(preprocessed_trace.domain),
    //             (None, Some(main_trace)) => Some(main_trace.domain),
    //             (None, None) => None,
    //         })
    //         .collect_vec();
    // }
}

pub type MachineTrace<'a, F, EF, Domain> = Vec<ChipTrace<'a, F, EF, Domain>>;

pub trait MachineTraceBuilder<'a> {
    fn new(chips: &'a [&ChipType]) -> Self;
}

impl<'a, F, EF, Domain> MachineTraceBuilder<'a> for MachineTrace<'a, F, EF, Domain>
where
    F: Field,
    EF: ExtensionField<F>,
    Domain: PolynomialSpace,
{
    fn new(chips: &'a [&ChipType]) -> Self {
        chips.iter().map(|chip| ChipTrace::new(chip)).collect_vec()
    }
}

pub trait MachineTraceLoader<'a, F, EF, Domain>
where
    F: Field,
    EF: ExtensionField<F>,
    Domain: PolynomialSpace,
{
    fn load_preprocessed<SC, P>(self, pcs: &P, traces: Vec<Option<RowMajorMatrix<F>>>) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>;

    fn load_main<SC, P>(self, pcs: &P, traces: Vec<Option<RowMajorMatrix<F>>>) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>;

    fn load_permutation<SC, P>(self, pcs: &P, traces: Vec<Option<RowMajorMatrix<EF>>>) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>;
}

impl<'a, F, EF, Domain> MachineTraceLoader<'a, F, EF, Domain> for MachineTrace<'a, F, EF, Domain>
where
    F: Field,
    EF: ExtensionField<F>,
    Domain: PolynomialSpace,
{
    fn load_preprocessed<SC, P>(mut self, pcs: &P, traces: Vec<Option<RowMajorMatrix<F>>>) -> Self
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

    fn load_main<SC, P>(mut self, pcs: &P, traces: Vec<Option<RowMajorMatrix<F>>>) -> Self
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

    fn load_permutation<SC, P>(mut self, pcs: &P, traces: Vec<Option<RowMajorMatrix<EF>>>) -> Self
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
    {
        let traces = load_traces::<SC, _>(pcs, traces);
        for (chip_trace, permutation) in self.iter_mut().zip_eq(traces) {
            chip_trace.permutation = permutation;
        }
        self
    }
}

pub trait MachineTraceCommiter<'a, F, EF, Domain>
where
    F: Field,
    EF: ExtensionField<F>,
    Domain: PolynomialSpace,
{
    fn commit_preprocessed<SC, P>(self, pcs: &P) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
        Domain: PolynomialSpace<Val = F>;

    fn commit_main<SC, P>(self, pcs: &P) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
        Domain: PolynomialSpace<Val = F>;

    fn commit_permutation<SC, P>(self, pcs: &P) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
        Domain: PolynomialSpace<Val = EF>;
}

impl<'a, F, EF, Domain> MachineTraceCommiter<'a, F, EF, Domain> for MachineTrace<'a, F, EF, Domain>
where
    F: Field,
    EF: ExtensionField<F>,
    Domain: PolynomialSpace,
{
    fn commit_preprocessed<SC, P>(self, pcs: &P) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
        Domain: PolynomialSpace<Val = F>,
    {
        let traces = self
            .into_iter()
            .map(|trace| trace.preprocessed)
            .collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_main<SC, P>(self, pcs: &P) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
        Domain: PolynomialSpace<Val = F>,
    {
        let traces = self.into_iter().map(|trace| trace.main).collect_vec();
        commit_traces::<SC>(pcs, traces)
    }

    fn commit_permutation<SC, P>(self, pcs: &P) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger, Domain = Domain>,
        SC: StarkGenericConfig<Pcs = P>,
        Domain: PolynomialSpace<Val = EF>,
    {
        let traces = self
            .into_iter()
            .map(|trace| trace.permutation)
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
                            matrix: trace,
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
        .flat_map(|mt| mt.map(|trace| (trace.domain, trace.matrix)))
        .collect();
    if !domains_and_traces.is_empty() {
        let (commit, data) = pcs.commit(domains_and_traces);
        (Some(commit), Some(data))
    } else {
        (None, None)
    }
}

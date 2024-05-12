use p3_commit::Pcs;
use p3_field::Field;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_uni_stark::{Com, Domain, StarkGenericConfig, Val};

use crate::{proof::PcsProverData, trace::Trace};

pub trait Commiter<P> {
    fn load_traces<F, SC>(
        &self,
        traces: Vec<Option<RowMajorMatrix<F>>>,
    ) -> Vec<Option<Trace<F, Domain<SC>>>>
    where
        F: Field,
        P: Pcs<SC::Challenge, SC::Challenger>,
        SC: StarkGenericConfig<Pcs = P>;

    fn commit_traces<SC>(
        &self,
        traces: Vec<Option<Trace<Val<SC>, Domain<SC>>>>,
    ) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger>,
        SC: StarkGenericConfig<Pcs = P>;
}

impl<P> Commiter<P> for P {
    fn load_traces<F, SC>(
        &self,
        traces: Vec<Option<RowMajorMatrix<F>>>,
    ) -> Vec<Option<Trace<F, Domain<SC>>>>
    where
        F: Field,
        P: Pcs<SC::Challenge, SC::Challenger>,
        SC: StarkGenericConfig<Pcs = P>,
    {
        traces
            .into_iter()
            .scan(0usize, |count, mt| {
                Some({
                    if let Some(trace) = mt {
                        let degree = trace.height();
                        if degree > 0 {
                            let domain = self.natural_domain_for_degree(degree);
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
        &self,
        traces: Vec<Option<Trace<Val<SC>, Domain<SC>>>>,
    ) -> (Option<Com<SC>>, Option<PcsProverData<SC>>)
    where
        P: Pcs<SC::Challenge, SC::Challenger>,
        SC: StarkGenericConfig<Pcs = P>,
    {
        let domains_and_traces: Vec<_> = traces
            .into_iter()
            .flat_map(|mt| mt.map(|trace| (trace.domain, trace.value)))
            .collect();
        if !domains_and_traces.is_empty() {
            let (commit, data) = self.commit(domains_and_traces);
            (Some(commit), Some(data))
        } else {
            (None, None)
        }
    }
}

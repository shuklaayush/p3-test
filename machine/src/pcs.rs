use itertools::Itertools;
use p3_commit::Pcs;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_uni_stark::{Com, StarkGenericConfig, Val};

use crate::{proof::ProverData, trace::ChipTrace};

pub trait Commiter<SC: StarkGenericConfig> {
    fn load_traces(
        &self,
        traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
    ) -> Vec<Option<ChipTrace<SC>>>;

    fn commit_traces(
        &self,
        traces: Vec<Option<ChipTrace<SC>>>,
    ) -> (Option<Com<SC>>, Option<ProverData<SC>>);
}

impl<P, SC> Commiter<SC> for P
where
    P: Pcs<SC::Challenge, SC::Challenger, ProverData = ProverData<SC>>,
    SC: StarkGenericConfig<Pcs = P>,
{
    fn load_traces(
        &self,
        traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
    ) -> Vec<Option<ChipTrace<SC>>> {
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

                            Some(ChipTrace {
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

    fn commit_traces(
        &self,
        traces: Vec<Option<ChipTrace<SC>>>,
    ) -> (Option<Com<SC>>, Option<ProverData<SC>>) {
        let domains_and_traces = traces
            .into_iter()
            .flat_map(|mt| mt.map(|trace| (trace.domain, trace.matrix)))
            .collect_vec();
        if !domains_and_traces.is_empty() {
            let (commit, data) = self.commit(domains_and_traces);
            (Some(commit), Some(data))
        } else {
            (None, None)
        }
    }
}

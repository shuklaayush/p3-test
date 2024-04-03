use p3_uni_stark::{Com, PcsProof, StarkGenericConfig};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(bound = "SC::Challenge: Serialize + DeserializeOwned")]
pub struct MachineProof<SC: StarkGenericConfig> {
    pub commitments: Commitments<Com<SC>>,
    pub opening_proof: PcsProof<SC>,
    pub chip_proofs: Vec<ChipProof<SC::Challenge>>,
}

#[derive(Serialize, Deserialize)]
pub struct Commitments<Com> {
    pub main_trace: Com,
    pub perm_trace: Com,
    pub quotient_chunks: Com,
}

#[derive(Serialize, Deserialize)]
pub struct ChipProof<Challenge> {
    pub degree_bits: usize,
    pub opened_values: OpenedValues<Challenge>,
    pub cumulative_sum: Challenge,
}

#[derive(Serialize, Deserialize)]
pub struct OpenedValues<Challenge> {
    pub preprocessed_local: Vec<Challenge>,
    pub preprocessed_next: Vec<Challenge>,
    pub trace_local: Vec<Challenge>,
    pub trace_next: Vec<Challenge>,
    pub permutation_local: Vec<Challenge>,
    pub permutation_next: Vec<Challenge>,
    pub quotient_chunks: Vec<Vec<Challenge>>,
}

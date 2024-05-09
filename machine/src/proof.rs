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
    pub cumulative_sum: Option<Challenge>,
}

#[derive(Serialize, Deserialize)]
pub struct OpenedValues<Challenge> {
    pub preprocessed: Option<AdjacentOpenedValues<Challenge>>,
    pub main: AdjacentOpenedValues<Challenge>,
    pub permutation: Option<AdjacentOpenedValues<Challenge>>,
    pub quotient_chunks: Vec<Vec<Challenge>>,
}

#[derive(Serialize, Deserialize)]
pub struct AdjacentOpenedValues<Challenge> {
    pub local: Vec<Challenge>,
    pub next: Vec<Challenge>,
}

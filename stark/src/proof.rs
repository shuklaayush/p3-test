use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Commitments<Com> {
    pub main: Com,
    pub permutation: Com,
    pub quotient_chunks: Com,
}

// TODO: Rename
#[derive(Serialize, Deserialize)]
pub struct ChipProof<Challenge> {
    pub degree: usize,
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

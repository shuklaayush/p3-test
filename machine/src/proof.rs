use alloc::vec::Vec;

use p3_commit::Pcs;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, Val};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use p3_air_util::{Commitments, InteractionAirProof};

pub type Com<SC> = <<SC as StarkGenericConfig>::Pcs as Pcs<
    <SC as StarkGenericConfig>::Challenge,
    <SC as StarkGenericConfig>::Challenger,
>>::Commitment;
pub type PcsProof<SC> = <<SC as StarkGenericConfig>::Pcs as Pcs<
    <SC as StarkGenericConfig>::Challenge,
    <SC as StarkGenericConfig>::Challenger,
>>::Proof;

pub type PcsProverData<SC> = <<SC as StarkGenericConfig>::Pcs as Pcs<
    <SC as StarkGenericConfig>::Challenge,
    <SC as StarkGenericConfig>::Challenger,
>>::ProverData;

#[derive(Serialize, Deserialize, Clone)]
#[serde(bound = "SC::Challenge: Serialize + DeserializeOwned")]
pub struct MachineProof<SC: StarkGenericConfig> {
    pub commitments: Commitments<Com<SC>>,
    pub opening_proof: PcsProof<SC>,
    pub chip_proofs: Vec<Option<InteractionAirProof<SC::Challenge>>>,
}

pub struct ProverPreprocessedData<SC: StarkGenericConfig> {
    pub traces: Vec<Option<RowMajorMatrix<Val<SC>>>>,
    pub data: Option<PcsProverData<SC>>,
    pub commitment: Option<Com<SC>>,
}

#[derive(Serialize, Deserialize)]
pub struct VerifierPreprocessedData<SC: StarkGenericConfig> {
    pub commitment: Com<SC>,
    // Index, degree
    pub degrees: Vec<(usize, usize)>,
}

pub struct ProvingKey<SC: StarkGenericConfig> {
    pub preprocessed: ProverPreprocessedData<SC>,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyingKey<SC: StarkGenericConfig> {
    pub preprocessed: Option<VerifierPreprocessedData<SC>>,
}

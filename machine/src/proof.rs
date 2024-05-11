use p3_commit::Pcs;
use p3_stark::{ChipProof, Commitments};
use p3_uni_stark::{Com, Domain, PcsProof, StarkGenericConfig, Val};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::trace::ChipTrace;

pub type PcsProverData<SC> = <<SC as StarkGenericConfig>::Pcs as Pcs<
    <SC as StarkGenericConfig>::Challenge,
    <SC as StarkGenericConfig>::Challenger,
>>::ProverData;

#[derive(Serialize, Deserialize)]
#[serde(bound = "SC::Challenge: Serialize + DeserializeOwned")]
pub struct MachineProof<SC: StarkGenericConfig> {
    pub commitments: Commitments<Com<SC>>,
    pub opening_proof: PcsProof<SC>,
    pub chip_proofs: Vec<ChipProof<SC::Challenge>>,
}

pub struct ProverData<SC: StarkGenericConfig> {
    pub data: PcsProverData<SC>,
    pub commitment: Com<SC>,
}

pub struct VerifierData<SC: StarkGenericConfig> {
    pub commitment: Com<SC>,
    pub degrees: Vec<usize>,
}

pub struct ProvingKey<SC: StarkGenericConfig> {
    pub preprocessed_data: Option<ProverData<SC>>,
    pub preprocessed_traces: Vec<Option<ChipTrace<Val<SC>, Domain<SC>>>>,
}

pub struct VerifyingKey<SC: StarkGenericConfig> {
    pub preprocessed_data: Option<VerifierData<SC>>,
}

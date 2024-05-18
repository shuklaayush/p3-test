use p3_interaction::{InteractionAir, InteractionChip};
use p3_stark::{
    debug::DebugConstraintBuilder, prover::ProverConstraintFolder, symbolic::SymbolicAirBuilder,
    verifier::VerifierConstraintFolder, AirDebug,
};

use p3_air::BaseAir;
use p3_uni_stark::{StarkGenericConfig, Val};
use std::fmt::{Debug, Display};

pub trait MachineChip<SC: StarkGenericConfig>:
    BaseAir<Val<SC>>
    + InteractionChip<Val<SC>>
    + AirDebug<Val<SC>, SC::Challenge>
    + Clone
    + Debug
    + Display
    + for<'a> InteractionAir<ProverConstraintFolder<'a, SC>>
    + for<'a> InteractionAir<VerifierConstraintFolder<'a, SC>>
    + for<'a> InteractionAir<SymbolicAirBuilder<Val<SC>>>
    + for<'a> InteractionAir<DebugConstraintBuilder<'a, Val<SC>, SC::Challenge>>
{
}

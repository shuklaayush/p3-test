use core::fmt::Display;
use p3_interaction::InteractionAir;
use p3_stark::{
    debug::DebugConstraintBuilder, prover::ProverConstraintFolder, symbolic::SymbolicAirBuilder,
    verifier::VerifierConstraintFolder, AirDebug,
};
use p3_uni_stark::{StarkGenericConfig, Val};

pub trait MachineChip<SC: StarkGenericConfig>:
// TODO: Remove clone
    Clone
    + Display
    + for<'a> InteractionAir<ProverConstraintFolder<'a, SC>>
    + for<'a> InteractionAir<VerifierConstraintFolder<'a, SC>>
    + for<'a> InteractionAir<SymbolicAirBuilder<Val<SC>>>
    + for<'a> InteractionAir<DebugConstraintBuilder<'a, Val<SC>, SC::Challenge>>
    // TODO: Put behind flag
    + AirDebug<Val<SC>, SC::Challenge>
{
}

use core::fmt::Display;
use p3_uni_stark::{StarkGenericConfig, Val};

#[cfg(feature = "trace-writer")]
use p3_air_util::AirTraceWriter;
use p3_air_util::{
    debug::DebugConstraintBuilder, prover::ProverConstraintFolder, symbolic::SymbolicAirBuilder,
    verifier::VerifierConstraintFolder,
};
use p3_interaction::Rap;

pub trait MachineChip<SC: StarkGenericConfig>:
// TODO: Remove clone
    Clone
    + Display
    + for<'a> Rap<ProverConstraintFolder<'a, SC>>
    + for<'a> Rap<VerifierConstraintFolder<'a, SC>>
    + for<'a> Rap<SymbolicAirBuilder<Val<SC>>>
    + for<'a> Rap<DebugConstraintBuilder<'a, Val<SC>, SC::Challenge>>
    // TODO: Put behind flag
    + AirTraceWriter<Val<SC>, SC::Challenge>
{
}

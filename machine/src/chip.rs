use core::fmt::Display;
use p3_uni_stark::{StarkGenericConfig, Val};
use std::fmt::Debug;

use p3_air_util::folders::{
    DebugConstraintBuilder, ProverConstraintFolder, SymbolicAirBuilder, TrackingConstraintBuilder,
    VerifierConstraintFolder,
};
#[cfg(feature = "trace-writer")]
use p3_air_util::TraceWriter;
use p3_interaction::Rap;

pub trait MachineChip<SC: StarkGenericConfig>:
// TODO: Remove clone
    Clone
    + Debug
    + Display
    + for<'a> Rap<ProverConstraintFolder<'a, SC>>
    + for<'a> Rap<VerifierConstraintFolder<'a, SC>>
    + for<'a> Rap<SymbolicAirBuilder<Val<SC>>>
    + for<'a> Rap<DebugConstraintBuilder<'a, Val<SC>, SC::Challenge>>
    // TODO: Put behind trace-writer flag
    + TraceWriter<Val<SC>, SC::Challenge>
    + for<'a> Rap<TrackingConstraintBuilder<'a, Val<SC>, SC::Challenge>>
{
}

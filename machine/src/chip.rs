use core::fmt::Display;
use p3_uni_stark::{StarkGenericConfig, Val};
use std::fmt::Debug;

#[cfg(feature = "trace-writer")]
use p3_air_util::folders::TrackingConstraintBuilder;
use p3_air_util::folders::{
    DebugConstraintBuilder, ProverConstraintFolder, SymbolicAirBuilder, VerifierConstraintFolder,
};
#[cfg(feature = "trace-writer")]
use p3_air_util::TraceWriter;
use p3_interaction::Rap;

// TODO: Remove clone
#[cfg(not(feature = "trace-writer"))]
pub trait Chip<SC>:
    Clone
    + Debug
    + Display
    + for<'a> Rap<ProverConstraintFolder<'a, SC>>
    + for<'a> Rap<VerifierConstraintFolder<'a, SC>>
    + for<'a> Rap<SymbolicAirBuilder<Val<SC>>>
    + for<'a> Rap<DebugConstraintBuilder<'a, Val<SC>, SC::Challenge>>
where
    SC: StarkGenericConfig,
{
}

#[cfg(feature = "trace-writer")]
pub trait Chip<SC>:
    Clone
    + Debug
    + Display
    + for<'a> Rap<ProverConstraintFolder<'a, SC>>
    + for<'a> Rap<VerifierConstraintFolder<'a, SC>>
    + for<'a> Rap<SymbolicAirBuilder<Val<SC>>>
    + for<'a> Rap<DebugConstraintBuilder<'a, Val<SC>, SC::Challenge>>
    + TraceWriter<Val<SC>, SC::Challenge>
    + for<'a> Rap<TrackingConstraintBuilder<'a, Val<SC>, SC::Challenge>>
where
    SC: StarkGenericConfig,
{
}

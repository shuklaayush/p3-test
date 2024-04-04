use p3_air::Air;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, SymbolicAirBuilder, Val};

use crate::debug_builder::DebugConstraintBuilder;
use crate::folder::ProverConstraintFolder;
use crate::interaction::{Interaction, InteractionType};

pub trait Chip<F: Field> {
    fn generate_trace(&self) -> RowMajorMatrix<F>;

    fn sends(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        vec![]
    }

    fn all_interactions(&self) -> Vec<(Interaction<F>, InteractionType)> {
        let mut interactions: Vec<(Interaction<F>, InteractionType)> = vec![];
        interactions.extend(self.sends().into_iter().map(|i| (i, InteractionType::Send)));
        interactions.extend(
            self.receives()
                .into_iter()
                .map(|i| (i, InteractionType::Receive)),
        );
        interactions
    }
}

pub trait MachineChip<SC: StarkGenericConfig>: Chip<Val<SC>> + for<'a> Air<ProverConstraintFolder<'a, SC>>
    // + for<'a> Air<VerifierConstraintFolder<'a, SC>>
    + for<'a> Air<SymbolicAirBuilder<Val<SC>>>
    + for<'a> Air<DebugConstraintBuilder<'a, SC>>
{
    fn trace_width(&self) -> usize {
        self.width()
    }
}

use p3_field::{AbstractField, ExtensionField, Field};
use p3_interaction::{Interaction, InteractionAir, InteractionAirBuilder, InteractionChip};
use p3_stark::{InteractionStark, Stark};

use p3_air::{Air, AirBuilder, BaseAir};
use p3_matrix::dense::RowMajorMatrix;
use std::fmt::{self, Display, Formatter};

use crate::{
    chips::keccak_permute::KeccakPermuteChip, chips::keccak_sponge::KeccakSpongeChip,
    chips::memory::MemoryChip, chips::merkle_tree::MerkleTreeChip,
    chips::range_checker::RangeCheckerChip, chips::xor::XorChip,
};

pub trait Chip<F: Field, EF: ExtensionField<F>>: Stark<F> + InteractionStark<F, EF> {}

// pub trait Chip<SC: StarkGenericConfig>:
//     for<'a> InteractionAir<ProverConstraintFolder<'a, SC>>
//     + for<'a> InteractionAir<VerifierConstraintFolder<'a, SC>>
//     + for<'a> InteractionAir<DebugConstraintBuilder<'a, SC>>

#[derive(Clone)]
pub enum ChipType {
    KeccakPermute(KeccakPermuteChip),
    KeccakSponge(KeccakSpongeChip),
    MerkleTree(MerkleTreeChip),
    Range8(RangeCheckerChip<256>),
    Xor(XorChip),
    Memory(MemoryChip),
}

impl Display for ChipType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ChipType::KeccakPermute(_) => write!(f, "KeccakPermute"),
            ChipType::KeccakSponge(_) => write!(f, "KeccakSponge"),
            ChipType::MerkleTree(_) => write!(f, "MerkleTree"),
            ChipType::Range8(_) => write!(f, "Range8"),
            ChipType::Xor(_) => write!(f, "Xor"),
            ChipType::Memory(_) => write!(f, "Memory"),
        }
    }
}

// TODO: Write a proc_macro for enum dispatch
impl<F: Field> BaseAir<F> for ChipType {
    fn width(&self) -> usize {
        match self {
            ChipType::KeccakPermute(chip) => <KeccakPermuteChip as BaseAir<F>>::width(chip),
            ChipType::KeccakSponge(chip) => <KeccakSpongeChip as BaseAir<F>>::width(chip),
            ChipType::MerkleTree(chip) => <MerkleTreeChip as BaseAir<F>>::width(chip),
            ChipType::Range8(chip) => <RangeCheckerChip<256> as BaseAir<F>>::width(chip),
            ChipType::Xor(chip) => <XorChip as BaseAir<F>>::width(chip),
            ChipType::Memory(chip) => <MemoryChip as BaseAir<F>>::width(chip),
        }
    }

    fn preprocessed_trace(&self) -> Option<RowMajorMatrix<F>> {
        match self {
            ChipType::KeccakPermute(chip) => {
                <KeccakPermuteChip as BaseAir<F>>::preprocessed_trace(chip)
            }
            ChipType::KeccakSponge(chip) => {
                <KeccakSpongeChip as BaseAir<F>>::preprocessed_trace(chip)
            }
            ChipType::MerkleTree(chip) => <MerkleTreeChip as BaseAir<F>>::preprocessed_trace(chip),
            ChipType::Range8(chip) => {
                <RangeCheckerChip<256> as BaseAir<F>>::preprocessed_trace(chip)
            }
            ChipType::Xor(chip) => <XorChip as BaseAir<F>>::preprocessed_trace(chip),
            ChipType::Memory(chip) => <MemoryChip as BaseAir<F>>::preprocessed_trace(chip),
        }
    }
}

impl<AB: AirBuilder> Air<AB> for ChipType {
    fn eval(&self, builder: &mut AB) {
        match self {
            ChipType::KeccakPermute(chip) => <KeccakPermuteChip as Air<AB>>::eval(chip, builder),
            ChipType::KeccakSponge(chip) => <KeccakSpongeChip as Air<AB>>::eval(chip, builder),
            ChipType::MerkleTree(chip) => <MerkleTreeChip as Air<AB>>::eval(chip, builder),
            ChipType::Range8(chip) => <RangeCheckerChip<256> as Air<AB>>::eval(chip, builder),
            ChipType::Xor(chip) => <XorChip as Air<AB>>::eval(chip, builder),
            ChipType::Memory(chip) => <MemoryChip as Air<AB>>::eval(chip, builder),
        }
    }
}

impl<F: AbstractField> InteractionChip<F> for ChipType {
    fn sends(&self) -> Vec<Interaction<F>> {
        match self {
            ChipType::KeccakPermute(chip) => <KeccakPermuteChip as InteractionChip<F>>::sends(chip),
            ChipType::KeccakSponge(chip) => <KeccakSpongeChip as InteractionChip<F>>::sends(chip),
            ChipType::MerkleTree(chip) => <MerkleTreeChip as InteractionChip<F>>::sends(chip),
            ChipType::Range8(chip) => <RangeCheckerChip<256> as InteractionChip<F>>::sends(chip),
            ChipType::Xor(chip) => <XorChip as InteractionChip<F>>::sends(chip),
            ChipType::Memory(chip) => <MemoryChip as InteractionChip<F>>::sends(chip),
        }
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        match self {
            ChipType::KeccakPermute(chip) => {
                <KeccakPermuteChip as InteractionChip<F>>::receives(chip)
            }
            ChipType::KeccakSponge(chip) => {
                <KeccakSpongeChip as InteractionChip<F>>::receives(chip)
            }
            ChipType::MerkleTree(chip) => <MerkleTreeChip as InteractionChip<F>>::receives(chip),
            ChipType::Range8(chip) => <RangeCheckerChip<256> as InteractionChip<F>>::receives(chip),
            ChipType::Xor(chip) => <XorChip as InteractionChip<F>>::receives(chip),
            ChipType::Memory(chip) => <MemoryChip as InteractionChip<F>>::receives(chip),
        }
    }
}

impl<AB: InteractionAirBuilder> InteractionAir<AB> for ChipType {}

use p3_field::{AbstractField, ExtensionField, Field, PrimeField32};
use p3_interaction::{Interaction, InteractionAir, InteractionAirBuilder, InteractionChip};
use p3_stark::AirDebug;

use p3_air::{Air, AirBuilder, BaseAir};
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, Val};
use std::fmt::{self, Debug, Display, Formatter};

use crate::{
    chip::MachineChip,
    chips::{
        keccak_permute::KeccakPermuteChip, keccak_sponge::KeccakSpongeChip, memory::MemoryChip,
        merkle_tree::MerkleTreeChip, range_checker::RangeCheckerChip, xor::XorChip,
    },
};

#[derive(Clone, Debug)]
pub enum KeccakMachineChip {
    KeccakPermute(KeccakPermuteChip),
    KeccakSponge(KeccakSpongeChip),
    MerkleTree(MerkleTreeChip),
    Range8(RangeCheckerChip<256>),
    Xor(XorChip),
    Memory(MemoryChip),
}

impl Display for KeccakMachineChip {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            KeccakMachineChip::KeccakPermute(_) => write!(f, "KeccakPermute"),
            KeccakMachineChip::KeccakSponge(_) => write!(f, "KeccakSponge"),
            KeccakMachineChip::MerkleTree(_) => write!(f, "MerkleTree"),
            KeccakMachineChip::Range8(_) => write!(f, "Range8"),
            KeccakMachineChip::Xor(_) => write!(f, "Xor"),
            KeccakMachineChip::Memory(_) => write!(f, "Memory"),
        }
    }
}

// TODO: Write a proc_macro for enum dispatch
impl<F: Field> BaseAir<F> for KeccakMachineChip {
    fn width(&self) -> usize {
        match self {
            KeccakMachineChip::KeccakPermute(chip) => {
                <KeccakPermuteChip as BaseAir<F>>::width(chip)
            }
            KeccakMachineChip::KeccakSponge(chip) => <KeccakSpongeChip as BaseAir<F>>::width(chip),
            KeccakMachineChip::MerkleTree(chip) => <MerkleTreeChip as BaseAir<F>>::width(chip),
            KeccakMachineChip::Range8(chip) => <RangeCheckerChip<256> as BaseAir<F>>::width(chip),
            KeccakMachineChip::Xor(chip) => <XorChip as BaseAir<F>>::width(chip),
            KeccakMachineChip::Memory(chip) => <MemoryChip as BaseAir<F>>::width(chip),
        }
    }

    fn preprocessed_trace(&self) -> Option<RowMajorMatrix<F>> {
        match self {
            KeccakMachineChip::KeccakPermute(chip) => {
                <KeccakPermuteChip as BaseAir<F>>::preprocessed_trace(chip)
            }
            KeccakMachineChip::KeccakSponge(chip) => {
                <KeccakSpongeChip as BaseAir<F>>::preprocessed_trace(chip)
            }
            KeccakMachineChip::MerkleTree(chip) => {
                <MerkleTreeChip as BaseAir<F>>::preprocessed_trace(chip)
            }
            KeccakMachineChip::Range8(chip) => {
                <RangeCheckerChip<256> as BaseAir<F>>::preprocessed_trace(chip)
            }
            KeccakMachineChip::Xor(chip) => <XorChip as BaseAir<F>>::preprocessed_trace(chip),
            KeccakMachineChip::Memory(chip) => <MemoryChip as BaseAir<F>>::preprocessed_trace(chip),
        }
    }
}

impl<AB: AirBuilder> Air<AB> for KeccakMachineChip {
    fn eval(&self, builder: &mut AB) {
        match self {
            KeccakMachineChip::KeccakPermute(chip) => {
                <KeccakPermuteChip as Air<AB>>::eval(chip, builder)
            }
            KeccakMachineChip::KeccakSponge(chip) => {
                <KeccakSpongeChip as Air<AB>>::eval(chip, builder)
            }
            KeccakMachineChip::MerkleTree(chip) => <MerkleTreeChip as Air<AB>>::eval(chip, builder),
            KeccakMachineChip::Range8(chip) => {
                <RangeCheckerChip<256> as Air<AB>>::eval(chip, builder)
            }
            KeccakMachineChip::Xor(chip) => <XorChip as Air<AB>>::eval(chip, builder),
            KeccakMachineChip::Memory(chip) => <MemoryChip as Air<AB>>::eval(chip, builder),
        }
    }
}

impl<F: AbstractField> InteractionChip<F> for KeccakMachineChip {
    fn sends(&self) -> Vec<Interaction<F>> {
        match self {
            KeccakMachineChip::KeccakPermute(chip) => {
                <KeccakPermuteChip as InteractionChip<F>>::sends(chip)
            }
            KeccakMachineChip::KeccakSponge(chip) => {
                <KeccakSpongeChip as InteractionChip<F>>::sends(chip)
            }
            KeccakMachineChip::MerkleTree(chip) => {
                <MerkleTreeChip as InteractionChip<F>>::sends(chip)
            }
            KeccakMachineChip::Range8(chip) => {
                <RangeCheckerChip<256> as InteractionChip<F>>::sends(chip)
            }
            KeccakMachineChip::Xor(chip) => <XorChip as InteractionChip<F>>::sends(chip),
            KeccakMachineChip::Memory(chip) => <MemoryChip as InteractionChip<F>>::sends(chip),
        }
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        match self {
            KeccakMachineChip::KeccakPermute(chip) => {
                <KeccakPermuteChip as InteractionChip<F>>::receives(chip)
            }
            KeccakMachineChip::KeccakSponge(chip) => {
                <KeccakSpongeChip as InteractionChip<F>>::receives(chip)
            }
            KeccakMachineChip::MerkleTree(chip) => {
                <MerkleTreeChip as InteractionChip<F>>::receives(chip)
            }
            KeccakMachineChip::Range8(chip) => {
                <RangeCheckerChip<256> as InteractionChip<F>>::receives(chip)
            }
            KeccakMachineChip::Xor(chip) => <XorChip as InteractionChip<F>>::receives(chip),
            KeccakMachineChip::Memory(chip) => <MemoryChip as InteractionChip<F>>::receives(chip),
        }
    }
}

impl<AB: InteractionAirBuilder> InteractionAir<AB> for KeccakMachineChip {
    fn preprocessed_width(&self) -> usize {
        match self {
            KeccakMachineChip::KeccakPermute(chip) => {
                <KeccakPermuteChip as InteractionAir<AB>>::preprocessed_width(chip)
            }
            KeccakMachineChip::KeccakSponge(chip) => {
                <KeccakSpongeChip as InteractionAir<AB>>::preprocessed_width(chip)
            }
            KeccakMachineChip::MerkleTree(chip) => {
                <MerkleTreeChip as InteractionAir<AB>>::preprocessed_width(chip)
            }
            KeccakMachineChip::Range8(chip) => {
                <RangeCheckerChip<256> as InteractionAir<AB>>::preprocessed_width(chip)
            }
            KeccakMachineChip::Xor(chip) => {
                <XorChip as InteractionAir<AB>>::preprocessed_width(chip)
            }
            KeccakMachineChip::Memory(chip) => {
                <MemoryChip as InteractionAir<AB>>::preprocessed_width(chip)
            }
        }
    }
}

impl<F: PrimeField32, EF: ExtensionField<F>> AirDebug<F, EF> for KeccakMachineChip {
    fn main_headers(&self) -> Vec<String> {
        match self {
            KeccakMachineChip::KeccakPermute(chip) => {
                <KeccakPermuteChip as AirDebug<F, EF>>::main_headers(chip)
            }
            KeccakMachineChip::KeccakSponge(chip) => {
                <KeccakSpongeChip as AirDebug<F, EF>>::main_headers(chip)
            }
            KeccakMachineChip::MerkleTree(chip) => {
                <MerkleTreeChip as AirDebug<F, EF>>::main_headers(chip)
            }
            KeccakMachineChip::Range8(chip) => {
                <RangeCheckerChip<256> as AirDebug<F, EF>>::main_headers(chip)
            }
            KeccakMachineChip::Xor(chip) => <XorChip as AirDebug<F, EF>>::main_headers(chip),
            KeccakMachineChip::Memory(chip) => <MemoryChip as AirDebug<F, EF>>::main_headers(chip),
        }
    }
}

impl<SC: StarkGenericConfig> MachineChip<SC> for KeccakMachineChip where Val<SC>: PrimeField32 {}

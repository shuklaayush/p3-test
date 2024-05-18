use p3_derive::EnumDispatch;
use std::fmt::Debug;

use crate::{
    chip::MachineChip,
    chips::{
        keccak_permute::KeccakPermuteChip, keccak_sponge::KeccakSpongeChip, memory::MemoryChip,
        merkle_tree::MerkleTreeChip, range_checker::RangeCheckerChip, xor::XorChip,
    },
};

#[derive(Clone, Debug, EnumDispatch)]
pub enum KeccakMachineChip {
    KeccakPermute(KeccakPermuteChip),
    KeccakSponge(KeccakSpongeChip),
    MerkleTree(MerkleTreeChip),
    Range8(RangeCheckerChip<256>),
    Xor(XorChip),
    Memory(MemoryChip),
}

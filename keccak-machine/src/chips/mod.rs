use p3_derive::EnumDispatch;
use std::fmt::Debug;

pub mod keccak_permute;
pub mod keccak_sponge;
pub mod memory;
pub mod merkle_tree;
pub mod range_checker;
pub mod xor;

use self::{
    keccak_permute::KeccakPermuteChip, keccak_sponge::KeccakSpongeChip, memory::MemoryChip,
    merkle_tree::MerkleTreeChip, range_checker::RangeCheckerChip, xor::XorChip,
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

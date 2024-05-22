use p3_field::PrimeField32;
use p3_machine::machine::Machine;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::{
    chips::KeccakMachineChip,
    chips::{
        keccak_permute::KeccakPermuteChip, keccak_sponge::KeccakSpongeChip, memory::MemoryChip,
        merkle_tree::MerkleTreeChip, range_checker::RangeCheckerChip, xor::XorChip,
    },
};

pub struct KeccakMachine {}

pub enum KeccakMachineBus {
    KeccakPermuteInput = 0,
    KeccakPermuteOutput = 1,
    KeccakPermuteDigest = 2,
    Range8 = 3,
    XorInput = 4,
    XorOutput = 5,
    Memory = 6,
}

impl<'a, SC> Machine<'a, SC, KeccakMachineChip> for KeccakMachine
where
    SC: StarkGenericConfig,
    Val<SC>: PrimeField32,
{
    fn chips(&self) -> Vec<KeccakMachineChip> {
        let keccak_permute_chip = KeccakPermuteChip {
            bus_keccak_permute_input: KeccakMachineBus::KeccakPermuteInput as usize,
            bus_keccak_permute_output: KeccakMachineBus::KeccakPermuteOutput as usize,
            bus_keccak_permute_digest_output: KeccakMachineBus::KeccakPermuteDigest as usize,
        };
        let keccak_sponge_chip = KeccakSpongeChip {
            bus_xor_input: KeccakMachineBus::XorInput as usize,
            bus_keccak_permute_input: KeccakMachineBus::KeccakPermuteInput as usize,
            bus_range_8: KeccakMachineBus::Range8 as usize,
            bus_memory: KeccakMachineBus::Memory as usize,
            bus_xor_output: KeccakMachineBus::XorOutput as usize,
            bus_keccak_permute_output: KeccakMachineBus::KeccakPermuteOutput as usize,
        };
        let merkle_tree_chip = MerkleTreeChip {
            bus_keccak_permute_input: KeccakMachineBus::KeccakPermuteInput as usize,
            bus_keccak_digest_output: KeccakMachineBus::KeccakPermuteDigest as usize,
        };
        let range_chip = RangeCheckerChip {
            bus_range_8: KeccakMachineBus::Range8 as usize,
        };
        let xor_chip = XorChip {
            bus_xor_input: KeccakMachineBus::XorInput as usize,
            bus_xor_output: KeccakMachineBus::XorOutput as usize,
        };
        let memory_chip = MemoryChip {
            bus_memory: KeccakMachineBus::Memory as usize,
            bus_range_8: KeccakMachineBus::Range8 as usize,
        };

        vec![
            KeccakMachineChip::KeccakPermute(keccak_permute_chip),
            KeccakMachineChip::KeccakSponge(keccak_sponge_chip),
            KeccakMachineChip::MerkleTree(merkle_tree_chip),
            KeccakMachineChip::Range8(range_chip),
            KeccakMachineChip::Xor(xor_chip),
            KeccakMachineChip::Memory(memory_chip),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{default_challenger, default_config, MyConfig},
        trace::generate_machine_trace,
    };

    use itertools::Itertools;
    use p3_keccak::KeccakF;
    use p3_machine::error::VerificationError;
    use p3_symmetric::{PseudoCompressionFunction, TruncatedPermutation};
    use rand::{random, thread_rng, Rng};
    use tracing_forest::{util::LevelFilter, ForestLayer};
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

    fn generate_digests(leaf_hashes: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
        let keccak = TruncatedPermutation::new(KeccakF {});
        let mut digests = vec![leaf_hashes.to_vec()];

        while let Some(last_level) = digests.last().cloned() {
            if last_level.len() == 1 {
                break;
            }

            let next_level = last_level
                .chunks_exact(2)
                .map(|chunk| keccak.compress([chunk[0], chunk[1]]))
                .collect();

            digests.push(next_level);
        }

        digests
    }

    #[test]
    fn test_machine_prove() -> Result<(), VerificationError> {
        let env_filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();

        Registry::default()
            .with(env_filter)
            .with(ForestLayer::default())
            .init();

        const NUM_BYTES: usize = 1000;
        let preimage = (0..NUM_BYTES).map(|_| random()).collect_vec();

        const HEIGHT: usize = 8;
        let leaf_hashes = (0..2u64.pow(HEIGHT as u32)).map(|_| random()).collect_vec();
        let digests = generate_digests(&leaf_hashes);

        let leaf_index = thread_rng().gen_range(0..leaf_hashes.len());
        let machine = KeccakMachine {};

        let (pk, vk) = machine.setup(&default_config());

        let config = default_config();
        let mut challenger = default_challenger();
        let traces = generate_machine_trace::<MyConfig>(preimage, digests, leaf_index);
        let proof = machine.prove(&config, &mut challenger, &pk, traces, &[]);

        let mut challenger = default_challenger();
        machine.verify(&config, &mut challenger, &vk, &proof, &[])
    }
}

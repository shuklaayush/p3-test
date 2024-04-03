mod columns;
mod generation;
mod round_flags;

use core::borrow::Borrow;

use p3_air::{Air, AirBuilder, BaseAir, VirtualPairCol};
use p3_field::{AbstractField, PrimeField64};
use p3_keccak_air::logic::{andn_gen, xor3_gen, xor_gen};
use p3_keccak_air::rc_value_bit;
use p3_keccak_air::{BITS_PER_LIMB, NUM_ROUNDS, U64_LIMBS};
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::MatrixRowSlices;
use p3_uni_stark::StarkGenericConfig;
use p3_uni_stark::Val;

use self::columns::{KeccakCols, KECCAK_COL_MAP, NUM_KECCAK_COLS};
use self::generation::generate_trace_rows;
use self::round_flags::eval_round_flags;
use crate::chip::{Chip, Interaction, MachineChip};

/// Assumes the field size is at least 16 bits.
pub struct KeccakPermuteChip {
    pub inputs: Vec<[u64; 25]>,
}

impl<F> BaseAir<F> for KeccakPermuteChip {
    fn width(&self) -> usize {
        NUM_KECCAK_COLS
    }
}

impl<AB: AirBuilder> Air<AB> for KeccakPermuteChip {
    fn eval(&self, builder: &mut AB) {
        eval_round_flags(builder);

        let main = builder.main();
        let local: &KeccakCols<AB::Var> = main.row_slice(0).borrow();
        let next: &KeccakCols<AB::Var> = main.row_slice(1).borrow();

        let mut builder = builder.when(local.is_real);

        // If this is not the final step, the export flag must be off.
        let final_step = local.step_flags[NUM_ROUNDS - 1];
        let not_final_step = AB::Expr::one() - final_step;

        // If this is not the final step, the local and next preimages must match.
        for y in 0..5 {
            for x in 0..5 {
                for limb in 0..U64_LIMBS {
                    builder
                        .when_transition()
                        .when(not_final_step.clone())
                        .assert_eq(local.preimage[y][x][limb], next.preimage[y][x][limb]);
                }
            }
        }

        // C'[x, z] = xor(C[x, z], C[x - 1, z], C[x + 1, z - 1]).
        for x in 0..5 {
            for z in 0..64 {
                let xor = xor3_gen::<AB::Expr>(
                    local.c[x][z].into(),
                    local.c[(x + 4) % 5][z].into(),
                    local.c[(x + 1) % 5][(z + 63) % 64].into(),
                );
                let c_prime = local.c_prime[x][z];
                builder.assert_eq(c_prime, xor);
            }
        }

        // Check that the input limbs are consistent with A' and D.
        // A[x, y, z] = xor(A'[x, y, z], D[x, y, z])
        //            = xor(A'[x, y, z], C[x - 1, z], C[x + 1, z - 1])
        //            = xor(A'[x, y, z], C[x, z], C'[x, z]).
        // The last step is valid based on the identity we checked above.
        // It isn't required, but makes this check a bit cleaner.
        for y in 0..5 {
            for x in 0..5 {
                let get_bit = |z| {
                    let a_prime: AB::Var = local.a_prime[y][x][z];
                    let c: AB::Var = local.c[x][z];
                    let c_prime: AB::Var = local.c_prime[x][z];
                    xor3_gen::<AB::Expr>(a_prime.into(), c.into(), c_prime.into())
                };

                for limb in 0..U64_LIMBS {
                    let a_limb = local.a[y][x][limb];
                    let computed_limb = (limb * BITS_PER_LIMB..(limb + 1) * BITS_PER_LIMB)
                        .rev()
                        .fold(AB::Expr::zero(), |acc, z| acc.double() + get_bit(z));
                    builder.assert_eq(computed_limb, a_limb);
                }
            }
        }

        // xor_{i=0}^4 A'[x, i, z] = C'[x, z], so for each x, z,
        // diff * (diff - 2) * (diff - 4) = 0, where
        // diff = sum_{i=0}^4 A'[x, i, z] - C'[x, z]
        for x in 0..5 {
            for z in 0..64 {
                let sum: AB::Expr = (0..5).map(|y| local.a_prime[y][x][z].into()).sum();
                let diff = sum - local.c_prime[x][z];
                let four = AB::Expr::from_canonical_u8(4);
                builder
                    .assert_zero(diff.clone() * (diff.clone() - AB::Expr::two()) * (diff - four));
            }
        }

        // A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
        for y in 0..5 {
            for x in 0..5 {
                let get_bit = |z| {
                    let andn = andn_gen::<AB::Expr>(
                        local.b((x + 1) % 5, y, z).into(),
                        local.b((x + 2) % 5, y, z).into(),
                    );
                    xor_gen::<AB::Expr>(local.b(x, y, z).into(), andn)
                };

                for limb in 0..U64_LIMBS {
                    let computed_limb = (limb * BITS_PER_LIMB..(limb + 1) * BITS_PER_LIMB)
                        .rev()
                        .fold(AB::Expr::zero(), |acc, z| acc.double() + get_bit(z));
                    builder.assert_eq(computed_limb, local.a_prime_prime[y][x][limb]);
                }
            }
        }

        // A'''[0, 0] = A''[0, 0] XOR RC
        for limb in 0..U64_LIMBS {
            let computed_a_prime_prime_0_0_limb = (limb * BITS_PER_LIMB
                ..(limb + 1) * BITS_PER_LIMB)
                .rev()
                .fold(AB::Expr::zero(), |acc, z| {
                    acc.double() + local.a_prime_prime_0_0_bits[z]
                });
            let a_prime_prime_0_0_limb = local.a_prime_prime[0][0][limb];
            builder.assert_eq(computed_a_prime_prime_0_0_limb, a_prime_prime_0_0_limb);
        }

        let get_xored_bit = |i| {
            let mut rc_bit_i = AB::Expr::zero();
            for r in 0..NUM_ROUNDS {
                let this_round = local.step_flags[r];
                let this_round_constant = AB::Expr::from_canonical_u8(rc_value_bit(r, i));
                rc_bit_i += this_round * this_round_constant;
            }

            xor_gen::<AB::Expr>(local.a_prime_prime_0_0_bits[i].into(), rc_bit_i)
        };

        for limb in 0..U64_LIMBS {
            let a_prime_prime_prime_0_0_limb = local.a_prime_prime_prime_0_0_limbs[limb];
            let computed_a_prime_prime_prime_0_0_limb = (limb * BITS_PER_LIMB
                ..(limb + 1) * BITS_PER_LIMB)
                .rev()
                .fold(AB::Expr::zero(), |acc, z| acc.double() + get_xored_bit(z));
            builder.assert_eq(
                computed_a_prime_prime_prime_0_0_limb,
                a_prime_prime_prime_0_0_limb,
            );
        }

        // Enforce that this round's output equals the next round's input.
        for x in 0..5 {
            for y in 0..5 {
                for limb in 0..U64_LIMBS {
                    let output = local.a_prime_prime_prime(x, y, limb);
                    let input = next.a[y][x][limb];
                    builder
                        .when_transition()
                        .when(not_final_step.clone())
                        .assert_eq(output, input);
                }
            }
        }
    }
}

impl<F: PrimeField64> Chip<F> for KeccakPermuteChip {
    fn generate_trace(&self) -> RowMajorMatrix<F> {
        generate_trace_rows(self.inputs.clone())
    }

    fn sends(&self) -> Vec<Interaction<F>> {
        let fields = (0..4)
            .flat_map(|i| {
                (0..U64_LIMBS)
                    .map(|limb| KECCAK_COL_MAP.a_prime_prime_prime(i % 5, i / 5, limb))
                    .collect::<Vec<_>>()
            })
            .map(VirtualPairCol::single_main)
            .collect();
        let is_real = VirtualPairCol::single_main(KECCAK_COL_MAP.step_flags[NUM_ROUNDS - 1]);
        let send = Interaction {
            fields,
            count: is_real,
            argument_index: 0,
        };
        vec![send]
    }

    fn receives(&self) -> Vec<Interaction<F>> {
        let fields = KECCAK_COL_MAP
            .preimage
            .into_iter()
            .flatten()
            .take(8)
            .flatten()
            .map(VirtualPairCol::single_main)
            .collect();
        let is_real = VirtualPairCol::single_main(KECCAK_COL_MAP.step_flags[0]);
        let receive = Interaction {
            fields,
            count: is_real,
            argument_index: 1,
        };
        vec![receive]
    }
}

impl<SC: StarkGenericConfig> MachineChip<SC> for KeccakPermuteChip where Val<SC>: PrimeField64 {}

#[cfg(test)]
mod tests {
    use super::*;

    use p3_baby_bear::BabyBear;
    use p3_challenger::{HashChallenger, SerializingChallenger32};
    use p3_commit::ExtensionMmcs;
    use p3_dft::Radix2DitParallel;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriConfig, TwoAdicFriPcs};
    use p3_keccak::Keccak256Hash;
    use p3_matrix::Matrix;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::{prove, verify, StarkConfig, VerificationError};
    use p3_util::log2_ceil_usize;
    use rand::random;

    const NUM_HASHES: usize = 10;

    #[test]
    fn test_keccak_prove() -> Result<(), VerificationError> {
        type Val = BabyBear;
        type Challenge = BinomialExtensionField<Val, 4>;

        type ByteHash = Keccak256Hash;
        type FieldHash = SerializingHasher32<ByteHash>;
        let byte_hash = ByteHash {};
        let field_hash = FieldHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<u8, ByteHash, 2, 32>;
        let compress = MyCompress::new(byte_hash);

        type ValMmcs = FieldMerkleTreeMmcs<Val, u8, FieldHash, MyCompress, 32>;
        let val_mmcs = ValMmcs::new(field_hash, compress);

        type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
        let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

        type Dft = Radix2DitParallel;
        let dft = Dft {};

        type Challenger = SerializingChallenger32<Val, HashChallenger<u8, ByteHash, 32>>;

        let inputs = (0..NUM_HASHES).map(|_| random()).collect::<Vec<_>>();
        let chip = KeccakPermuteChip { inputs };
        let trace = chip.generate_trace();

        let fri_config = FriConfig {
            log_blowup: 2,
            num_queries: 42,
            proof_of_work_bits: 16,
            mmcs: challenge_mmcs,
        };
        type Pcs = TwoAdicFriPcs<Val, Dft, ValMmcs, ChallengeMmcs>;
        let pcs = Pcs::new(log2_ceil_usize(trace.height()), dft, val_mmcs, fri_config);

        type MyConfig = StarkConfig<Pcs, Challenge, Challenger>;
        let config = MyConfig::new(pcs);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        let proof = prove(&config, &chip, &mut challenger, trace, &vec![]);

        let mut challenger = Challenger::from_hasher(vec![], byte_hash);
        verify(&config, &chip, &mut challenger, &proof, &vec![])
    }
}

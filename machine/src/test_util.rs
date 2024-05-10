use p3_air::Air;
use p3_field::PrimeField32;
use p3_stark::Stark;
use p3_uni_stark::{
    prove, verify, DebugConstraintBuilder, SymbolicAirBuilder, Val, VerificationError,
};
use p3_uni_stark::{ProverConstraintFolder, VerifierConstraintFolder};

use crate::config::{default_challenger, default_config, MyConfig};

#[cfg(debug_assertions)]
pub(crate) fn prove_and_verify<A>(
    air: &A,
    public_values: Vec<Val<MyConfig>>,
) -> Result<(), VerificationError>
where
    Val<MyConfig>: PrimeField32,
    A: Stark<Val<MyConfig>>
        + for<'a> Air<ProverConstraintFolder<'a, MyConfig>>
        + for<'a> Air<VerifierConstraintFolder<'a, MyConfig>>
        + for<'a> Air<SymbolicAirBuilder<Val<MyConfig>>>
        + for<'a> Air<DebugConstraintBuilder<'a, Val<MyConfig>>>,
{
    let config = default_config();

    let trace = air.generate_trace();

    let mut challenger = default_challenger();
    let proof = prove(&config, air, &mut challenger, trace, &public_values);

    let mut challenger = default_challenger();
    verify(&config, air, &mut challenger, &proof, &public_values)
}

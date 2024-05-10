use tiny_keccak::keccakf;

use super::columns::{KECCAK_WIDTH_BYTES, KECCAK_WIDTH_U16S};

/// Like tiny-keccak's `keccakf`, but deals with bytes instead of `u64` limbs.
pub(crate) fn keccakf_u8s(state_u8s: &mut [u8; KECCAK_WIDTH_BYTES]) {
    let mut state_u64s: [u64; 25] =
        core::array::from_fn(|i| u64::from_le_bytes(state_u8s[i * 8..][..8].try_into().unwrap()));
    keccakf(&mut state_u64s);
    *state_u8s = core::array::from_fn(|i| {
        let u64_limb = state_u64s[i / 8];
        u64_limb.to_le_bytes()[i % 8]
    });
}

/// Like tiny-keccak's `keccakf`, but deals with `u16` limbs instead of `u64`
/// limbs.
pub(crate) fn keccakf_u16s(state_u16s: &mut [u16; KECCAK_WIDTH_U16S]) {
    let mut state_u64s: [u64; 25] = core::array::from_fn(|i| {
        state_u16s[i * 4..(i + 1) * 4]
            .iter()
            .rev()
            .fold(0, |acc, &x| (acc << 16) | x as u64)
    });
    keccakf(&mut state_u64s);
    *state_u16s = core::array::from_fn(|i| {
        let u64_limb = state_u64s[i / 4];
        let shift = 16 * (i % 4);
        (u64_limb >> shift) as u16
    });
}

use core::fmt::{self, Debug, Display, Formatter};
use num_bigint::BigUint;
use std::hash::{Hash, Hasher};
use std::iter::{Product, Sum};
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

use p3_field::{AbstractField, Field, Packable, PrimeField, PrimeField32, PrimeField64};
use serde::{Deserialize, Deserializer, Serialize};

use super::fixed_set::FixedSet;

#[derive(Default, Clone, Copy)]
pub struct TrackedField<F: Field, const SET_SIZE: usize> {
    pub value: F,
    pub origin: FixedSet<SET_SIZE>,
}

impl<F: Field, const SET_SIZE: usize> TrackedField<F, SET_SIZE> {
    pub fn new(value: F, origin: FixedSet<SET_SIZE>) -> Self {
        Self { value, origin }
    }

    pub fn new_single(value: F, origin: usize) -> Self {
        let mut origin_set = FixedSet::new();
        origin_set.insert(origin);
        Self {
            value,
            origin: origin_set,
        }
    }
}

impl<F: Field, const SET_SIZE: usize> Eq for TrackedField<F, SET_SIZE> {}

impl<F: Field, const SET_SIZE: usize> PartialEq for TrackedField<F, SET_SIZE> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

// TODO: Is this required?
impl<F: Field, const SET_SIZE: usize> Hash for TrackedField<F, SET_SIZE> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<F: Field, const SET_SIZE: usize> Display for TrackedField<F, SET_SIZE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl<F: Field, const SET_SIZE: usize> Debug for TrackedField<F, SET_SIZE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

impl<F: Field, const SET_SIZE: usize> Serialize for TrackedField<F, SET_SIZE> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<'de, F: Field, const SET_SIZE: usize> Deserialize<'de> for TrackedField<F, SET_SIZE> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        F::deserialize(d).map(|value| Self {
            value,
            origin: FixedSet::default(),
        })
    }
}

impl<F: Field, const SET_SIZE: usize> Packable for TrackedField<F, SET_SIZE> {}

impl<F: Field, const SET_SIZE: usize> AbstractField for TrackedField<F, SET_SIZE> {
    type F = Self;

    fn zero() -> Self {
        Self {
            value: F::zero(),
            origin: FixedSet::default(),
        }
    }
    fn one() -> Self {
        Self {
            value: F::one(),
            origin: FixedSet::default(),
        }
    }

    fn two() -> Self {
        Self {
            value: F::two(),
            origin: FixedSet::default(),
        }
    }

    fn neg_one() -> Self {
        Self {
            value: F::neg_one(),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_f(f: Self::F) -> Self {
        f
    }

    #[inline]
    fn from_bool(b: bool) -> Self {
        Self {
            value: F::from_canonical_u32(b as u32),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_canonical_u8(n: u8) -> Self {
        Self {
            value: F::from_canonical_u8(n),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_canonical_u16(n: u16) -> Self {
        Self {
            value: F::from_canonical_u16(n),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_canonical_u32(n: u32) -> Self {
        Self {
            value: F::from_canonical_u32(n),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_canonical_u64(n: u64) -> Self {
        Self {
            value: F::from_canonical_u64(n),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_canonical_usize(n: usize) -> Self {
        Self {
            value: F::from_canonical_usize(n),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_wrapped_u32(n: u32) -> Self {
        Self {
            value: F::from_wrapped_u32(n),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn from_wrapped_u64(n: u64) -> Self {
        Self {
            value: F::from_wrapped_u64(n),
            origin: FixedSet::default(),
        }
    }

    #[inline]
    fn generator() -> Self {
        Self {
            value: F::generator(),
            origin: FixedSet::default(),
        }
    }
}

impl<F: Field, const SET_SIZE: usize> Field for TrackedField<F, SET_SIZE> {
    type Packing = Self;

    #[inline]
    fn mul_2exp_u64(&self, exp: u64) -> Self {
        Self {
            value: self.value.mul_2exp_u64(exp),
            origin: self.origin,
        }
    }

    fn try_inverse(&self) -> Option<Self> {
        let TrackedField { value, origin } = self;

        value.try_inverse().map(|v| Self {
            value: v,
            origin: *origin,
        })
    }

    #[inline]
    fn halve(&self) -> Self {
        Self {
            value: self.value.halve(),
            origin: self.origin,
        }
    }

    #[inline]
    fn order() -> BigUint {
        F::order()
    }
}

impl<F: Field, const SET_SIZE: usize> Add for TrackedField<F, SET_SIZE> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value + rhs.value,
            origin: self.origin | rhs.origin,
        }
    }
}

impl<F: Field, const SET_SIZE: usize> AddAssign for TrackedField<F, SET_SIZE> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<F: Field, const SET_SIZE: usize> Sum for TrackedField<F, SET_SIZE> {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                value: F::zero(),
                origin: FixedSet::default(),
            },
            |mut acc, item| {
                acc.value += item.value;
                acc.origin = acc.origin | item.origin;
                acc
            },
        )
    }
}

impl<F: Field, const SET_SIZE: usize> Sub for TrackedField<F, SET_SIZE> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value - rhs.value,
            origin: self.origin | rhs.origin,
        }
    }
}

impl<F: Field, const SET_SIZE: usize> SubAssign for TrackedField<F, SET_SIZE> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<F: Field, const SET_SIZE: usize> Neg for TrackedField<F, SET_SIZE> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::zero() - self
    }
}

impl<F: Field, const SET_SIZE: usize> Mul for TrackedField<F, SET_SIZE> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
            origin: self.origin | rhs.origin,
        }
    }
}

impl<F: Field, const SET_SIZE: usize> MulAssign for TrackedField<F, SET_SIZE> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: Field, const SET_SIZE: usize> Product for TrackedField<F, SET_SIZE> {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Self {
                value: F::zero(),
                origin: FixedSet::default(),
            },
            |mut acc, item| {
                acc.value *= item.value;
                acc.origin = acc.origin | item.origin;
                acc
            },
        )
    }
}

impl<F: Field, const SET_SIZE: usize> Div for TrackedField<F, SET_SIZE> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    #[inline]
    fn div(self, rhs: Self) -> Self {
        self * rhs.inverse()
    }
}

impl<F: Field, const SET_SIZE: usize> From<F> for TrackedField<F, SET_SIZE> {
    fn from(value: F) -> Self {
        Self {
            value,
            origin: FixedSet::default(),
        }
    }
}

impl<F: PrimeField32, const SET_SIZE: usize> PartialOrd for TrackedField<F, SET_SIZE> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.value.cmp(&other.value))
    }
}

impl<F: PrimeField32, const SET_SIZE: usize> Ord for TrackedField<F, SET_SIZE> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<F: PrimeField32, const SET_SIZE: usize> PrimeField for TrackedField<F, SET_SIZE> {
    fn as_canonical_biguint(&self) -> BigUint {
        self.value.as_canonical_biguint()
    }
}

impl<F: PrimeField32, const SET_SIZE: usize> PrimeField64 for TrackedField<F, SET_SIZE> {
    const ORDER_U64: u64 = F::ORDER_U64;

    #[inline]
    fn as_canonical_u64(&self) -> u64 {
        self.value.as_canonical_u64()
    }
}

impl<F: PrimeField32, const SET_SIZE: usize> PrimeField32 for TrackedField<F, SET_SIZE> {
    const ORDER_U32: u32 = F::ORDER_U32;

    #[inline]
    fn as_canonical_u32(&self) -> u32 {
        self.value.as_canonical_u32()
    }
}

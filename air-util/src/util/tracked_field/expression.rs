use core::fmt::Debug;
use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::collections::BTreeSet;

use p3_field::AbstractField;

use super::variable::Entry;

#[derive(Clone, Debug)]
pub struct TrackedFieldExpression<F: AbstractField> {
    pub value: F,
    pub origin: BTreeSet<Entry>,
}

impl<F: AbstractField> Default for TrackedFieldExpression<F> {
    fn default() -> Self {
        Self {
            value: F::zero(),
            origin: BTreeSet::new(),
        }
    }
}

impl<F: AbstractField> From<F> for TrackedFieldExpression<F> {
    fn from(value: F) -> Self {
        Self {
            value,
            origin: BTreeSet::new(),
        }
    }
}

impl<F: AbstractField> Add for TrackedFieldExpression<F> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value + rhs.value,
            origin: &self.origin | &rhs.origin,
        }
    }
}

impl<F: AbstractField> Add<F> for TrackedFieldExpression<F> {
    type Output = Self;

    fn add(self, rhs: F) -> Self {
        self + Self::from(rhs)
    }
}

impl<F: AbstractField> AddAssign for TrackedFieldExpression<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs;
    }
}

impl<F: AbstractField> AddAssign<F> for TrackedFieldExpression<F> {
    fn add_assign(&mut self, rhs: F) {
        *self += Self::from(rhs);
    }
}

impl<F: AbstractField> Sum for TrackedFieldExpression<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::zero())
    }
}

impl<F: AbstractField> Sum<F> for TrackedFieldExpression<F> {
    fn sum<I: Iterator<Item = F>>(iter: I) -> Self {
        iter.map(|x| Self::from(x)).sum()
    }
}

impl<F: AbstractField> Sub for TrackedFieldExpression<F> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value - rhs.value,
            origin: &self.origin | &rhs.origin,
        }
    }
}

impl<F: AbstractField> Sub<F> for TrackedFieldExpression<F> {
    type Output = Self;

    fn sub(self, rhs: F) -> Self {
        self - Self::from(rhs)
    }
}

impl<F: AbstractField> SubAssign for TrackedFieldExpression<F> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone() - rhs;
    }
}

impl<F: AbstractField> SubAssign<F> for TrackedFieldExpression<F> {
    fn sub_assign(&mut self, rhs: F) {
        *self -= Self::from(rhs);
    }
}

impl<F: AbstractField> Neg for TrackedFieldExpression<F> {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            value: -self.value,
            origin: self.origin,
        }
    }
}

impl<F: AbstractField> Mul for TrackedFieldExpression<F> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
            origin: &self.origin | &rhs.origin,
        }
    }
}

impl<F: AbstractField> Mul<F> for TrackedFieldExpression<F> {
    type Output = Self;

    fn mul(self, rhs: F) -> Self {
        self * Self::from(rhs)
    }
}

impl<F: AbstractField> MulAssign for TrackedFieldExpression<F> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.clone() * rhs;
    }
}

impl<F: AbstractField> MulAssign<F> for TrackedFieldExpression<F> {
    fn mul_assign(&mut self, rhs: F) {
        *self *= Self::from(rhs);
    }
}

impl<F: AbstractField> Product for TrackedFieldExpression<F> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::one())
    }
}

impl<F: AbstractField> Product<F> for TrackedFieldExpression<F> {
    fn product<I: Iterator<Item = F>>(iter: I) -> Self {
        iter.map(|x| Self::from(x)).product()
    }
}

impl<F: AbstractField> AbstractField for TrackedFieldExpression<F> {
    type F = F::F;

    fn zero() -> Self {
        Self::from(F::zero())
    }
    fn one() -> Self {
        Self::from(F::one())
    }
    fn two() -> Self {
        Self::from(F::two())
    }
    fn neg_one() -> Self {
        Self::from(F::neg_one())
    }

    #[inline]
    fn from_f(f: Self::F) -> Self {
        Self {
            value: F::from_f(f),
            origin: BTreeSet::new(),
        }
    }

    fn from_bool(b: bool) -> Self {
        Self::from(F::from_bool(b))
    }

    fn from_canonical_u8(n: u8) -> Self {
        Self::from(F::from_canonical_u8(n))
    }

    fn from_canonical_u16(n: u16) -> Self {
        Self::from(F::from_canonical_u16(n))
    }

    fn from_canonical_u32(n: u32) -> Self {
        Self::from(F::from_canonical_u32(n))
    }

    fn from_canonical_u64(n: u64) -> Self {
        Self::from(F::from_canonical_u64(n))
    }

    fn from_canonical_usize(n: usize) -> Self {
        Self::from(F::from_canonical_usize(n))
    }

    fn from_wrapped_u32(n: u32) -> Self {
        Self::from(F::from_wrapped_u32(n))
    }

    fn from_wrapped_u64(n: u64) -> Self {
        Self::from(F::from_wrapped_u64(n))
    }

    fn generator() -> Self {
        Self::from(F::generator())
    }
}

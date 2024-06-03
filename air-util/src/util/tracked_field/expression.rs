use core::fmt::Debug;
use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::collections::BTreeSet;

use p3_field::{AbstractField, Field};

#[derive(Clone, Debug)]
pub struct TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    pub value: F,
    pub origin: BTreeSet<E>,
}

impl<F, E> Default for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn default() -> Self {
        Self {
            value: F::zero(),
            origin: BTreeSet::new(),
        }
    }
}

impl<F, E> From<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn from(value: F) -> Self {
        Self {
            value,
            origin: BTreeSet::new(),
        }
    }
}

impl<F, E> Add for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let mut origin = BTreeSet::new();
        if !self.value.is_zero() {
            origin = origin.union(&self.origin).cloned().collect();
        }
        if !rhs.value.is_zero() {
            origin = origin.union(&rhs.origin).cloned().collect();
        }
        Self {
            value: self.value + rhs.value,
            origin,
        }
    }
}

impl<F, E> Add<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn add(self, rhs: F) -> Self {
        self + Self::from(rhs)
    }
}

impl<F, E> AddAssign for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs;
    }
}

impl<F, E> AddAssign<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn add_assign(&mut self, rhs: F) {
        *self += Self::from(rhs);
    }
}

impl<F, E> Sum for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::zero())
    }
}

impl<F, E> Sum<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn sum<I: Iterator<Item = F>>(iter: I) -> Self {
        iter.map(|x| Self::from(x)).sum()
    }
}

impl<F, E> Sub for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        let mut origin = BTreeSet::new();
        if !self.value.is_zero() {
            origin = origin.union(&self.origin).cloned().collect();
        }
        if !rhs.value.is_zero() {
            origin = origin.union(&rhs.origin).cloned().collect();
        }
        Self {
            value: self.value - rhs.value,
            origin,
        }
    }
}

impl<F, E> Sub<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn sub(self, rhs: F) -> Self {
        self - Self::from(rhs)
    }
}

impl<F, E> SubAssign for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone() - rhs;
    }
}

impl<F, E> SubAssign<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn sub_assign(&mut self, rhs: F) {
        *self -= Self::from(rhs);
    }
}

impl<F, E> Neg for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            value: -self.value,
            origin: self.origin,
        }
    }
}

impl<F, E> Mul for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let mut origin = BTreeSet::new();
        match (self.value.is_zero(), rhs.value.is_zero()) {
            (true, false) => {
                origin = origin.union(&self.origin).cloned().collect();
            }
            (false, true) => {
                origin = origin.union(&rhs.origin).cloned().collect();
            }
            (false, false) => {
                // Both
                origin = origin.union(&self.origin).cloned().collect();
                origin = origin.union(&rhs.origin).cloned().collect();
            }
            (true, true) => {
                // Either?
                origin = origin.union(&self.origin).cloned().collect();
                origin = origin.union(&rhs.origin).cloned().collect();
            }
        }
        Self {
            value: self.value * rhs.value,
            origin,
        }
    }
}

impl<F, E> Mul<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn mul(self, rhs: F) -> Self {
        self * Self::from(rhs)
    }
}

impl<F, E> MulAssign for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.clone() * rhs;
    }
}

impl<F, E> MulAssign<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn mul_assign(&mut self, rhs: F) {
        *self *= Self::from(rhs);
    }
}

impl<F, E> Product for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::one())
    }
}

impl<F, E> Product<F> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn product<I: Iterator<Item = F>>(iter: I) -> Self {
        iter.map(|x| Self::from(x)).product()
    }
}

impl<F, E> AbstractField for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
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

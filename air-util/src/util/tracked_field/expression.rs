use alloc::collections::BTreeSet;
use core::fmt::Debug;
use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use p3_field::{AbstractField, Field};

#[derive(Clone, Debug)]
pub struct TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    pub value: F,
    pub value_origin: BTreeSet<E>,
    pub constraint_origin: BTreeSet<E>,
}

impl<F, E> Default for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn default() -> Self {
        Self {
            value: F::zero(),
            value_origin: BTreeSet::new(),
            constraint_origin: BTreeSet::new(),
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
            value_origin: BTreeSet::new(),
            constraint_origin: BTreeSet::new(),
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
        let mut value_origin = BTreeSet::new();
        match (self.value.is_zero(), rhs.value.is_zero()) {
            (false, true) => {
                value_origin = value_origin.union(&self.value_origin).cloned().collect();
            }
            (true, false) => {
                value_origin = value_origin.union(&rhs.value_origin).cloned().collect();
            }
            (_, _) => {
                // Both or either
                value_origin = value_origin.union(&self.value_origin).cloned().collect();
                value_origin = value_origin.union(&rhs.value_origin).cloned().collect();
            }
        }
        Self {
            value: self.value + rhs.value,
            value_origin,
            constraint_origin: &self.constraint_origin | &rhs.constraint_origin,
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
        let mut value_origin = BTreeSet::new();
        match (self.value.is_zero(), rhs.value.is_zero()) {
            (false, true) => {
                value_origin = value_origin.union(&self.value_origin).cloned().collect();
            }
            (true, false) => {
                value_origin = value_origin.union(&rhs.value_origin).cloned().collect();
            }
            (_, _) => {
                // Both or either
                value_origin = value_origin.union(&self.value_origin).cloned().collect();
                value_origin = value_origin.union(&rhs.value_origin).cloned().collect();
            }
        }
        Self {
            value: self.value - rhs.value,
            value_origin,
            constraint_origin: &self.constraint_origin | &rhs.constraint_origin,
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
            value_origin: self.value_origin,
            constraint_origin: self.constraint_origin,
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
        let mut value_origin = BTreeSet::new();
        let mut constraint_origin = BTreeSet::new();
        match (self.value.is_zero(), rhs.value.is_zero()) {
            (true, false) => {
                value_origin = value_origin.union(&self.value_origin).cloned().collect();
                constraint_origin = constraint_origin
                    .union(&self.constraint_origin)
                    .cloned()
                    .collect();
            }
            (false, true) => {
                value_origin = value_origin.union(&rhs.value_origin).cloned().collect();
                constraint_origin = constraint_origin
                    .union(&rhs.constraint_origin)
                    .cloned()
                    .collect();
            }
            (true, true) => {
                // Either
                value_origin = &self.value_origin | &rhs.value_origin;
            }
            (false, false) => {
                // Both
                value_origin = &self.value_origin | &rhs.value_origin;
                constraint_origin = &self.constraint_origin | &rhs.constraint_origin;
            }
        }
        Self {
            value: self.value * rhs.value,
            value_origin,
            constraint_origin,
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
            value_origin: BTreeSet::new(),
            constraint_origin: BTreeSet::new(),
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

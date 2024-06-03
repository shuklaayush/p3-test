use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::fmt::Debug;
use std::marker::PhantomData;

use p3_field::{AbstractExtensionField, AbstractField, ExtensionField, Field};

use super::TrackedFieldExpression;

// TODO: Ideally we don't need this struct and can just use TrackedFieldExpression<EF, E>
//       but running into trait bounds issues
#[derive(Clone, Debug)]
pub struct TrackedExtensionFieldExpression<F, EF, E>(
    pub TrackedFieldExpression<EF, E>,
    pub PhantomData<F>,
)
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord;

impl<F, EF, E> Default for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn default() -> Self {
        Self::from(TrackedFieldExpression::<F, E>::default())
    }
}

impl<F, EF, E> From<F> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn from(value: F) -> Self {
        Self(TrackedFieldExpression::from(EF::from(value)), PhantomData)
    }
}

impl<F, EF, E> From<TrackedFieldExpression<F, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn from(value: TrackedFieldExpression<F, E>) -> Self {
        Self::from(value.value)
    }
}

impl<F, EF, E> Add for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, PhantomData)
    }
}

impl<F, EF, E> Add<TrackedFieldExpression<F, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn add(self, rhs: TrackedFieldExpression<F, E>) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl<F, EF, E> AddAssign for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs;
    }
}

impl<F, EF, E> AddAssign<TrackedFieldExpression<F, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn add_assign(&mut self, rhs: TrackedFieldExpression<F, E>) {
        *self = self.clone() + rhs;
    }
}

impl<F, EF, E> Sum for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::zero())
    }
}

impl<F, EF, E> Sub for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0, PhantomData)
    }
}

impl<F, EF, E> Sub<TrackedFieldExpression<F, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn sub(self, rhs: TrackedFieldExpression<F, E>) -> Self::Output {
        self - Self::from(rhs)
    }
}

impl<F, EF, E> SubAssign for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone() - rhs;
    }
}

impl<F, EF, E> SubAssign<TrackedFieldExpression<F, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn sub_assign(&mut self, rhs: TrackedFieldExpression<F, E>) {
        *self = self.clone() - rhs;
    }
}

impl<F, EF, E> Neg for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0, PhantomData)
    }
}

impl<F, EF, E> Mul for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0, PhantomData)
    }
}

impl<F, EF, E> Mul<TrackedFieldExpression<F, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn mul(self, rhs: TrackedFieldExpression<F, E>) -> Self::Output {
        self * Self::from(rhs)
    }
}

impl<F, EF, E> MulAssign for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.clone() * rhs;
    }
}

impl<F, EF, E> MulAssign<TrackedFieldExpression<F, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn mul_assign(&mut self, rhs: TrackedFieldExpression<F, E>) {
        *self = self.clone() * rhs;
    }
}

impl<F, EF, E> Product for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::one())
    }
}
impl<F, EF, E> AbstractField for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    type F = EF::F;

    fn zero() -> Self {
        Self::from(F::zero())
    }

    fn one() -> Self {
        Self::from(F::one())
    }

    fn two() -> Self {
        Self::from(F::one())
    }

    fn neg_one() -> Self {
        Self::from(F::neg_one())
    }

    fn from_f(f: Self::F) -> Self {
        Self(TrackedFieldExpression::from_f(f), PhantomData)
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

impl<F, EF, E> AbstractExtensionField<TrackedFieldExpression<F, E>>
    for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    const D: usize = EF::D;

    fn from_base(b: TrackedFieldExpression<F, E>) -> Self {
        Self::from(b)
    }

    fn from_base_slice(bs: &[TrackedFieldExpression<F, E>]) -> Self {
        let bs = bs.iter().map(|b| b.value.clone()).collect::<Vec<_>>();
        let value = EF::from_base_slice(&bs);
        Self(TrackedFieldExpression::from(value), PhantomData)
    }

    fn from_base_fn<FN: FnMut(usize) -> TrackedFieldExpression<F, E>>(mut f: FN) -> Self {
        let value = EF::from_base_fn(|i| f(i).value.clone());
        Self(TrackedFieldExpression::from(value), PhantomData)
    }

    fn as_base_slice(&self) -> &[TrackedFieldExpression<F, E>] {
        unimplemented!("TrackedExtensionFieldExpression::as_base_slice")
    }
}

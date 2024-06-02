use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::marker::PhantomData;

use p3_field::{AbstractExtensionField, AbstractField};

use super::TrackedFieldExpression;

// TODO: Ideally we don't need this struct and can just use TrackedFieldExpression<EF>
//       but running into trait bounds issues
#[derive(Clone, Debug)]
pub struct TrackedExtensionFieldExpression<F, EF>(
    pub TrackedFieldExpression<EF>,
    pub PhantomData<F>,
)
where
    F: AbstractField,
    EF: AbstractExtensionField<F>;

impl<F, EF> Default for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn default() -> Self {
        Self::from(TrackedFieldExpression::<F>::default())
    }
}

impl<F, EF> From<F> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn from(value: F) -> Self {
        Self(TrackedFieldExpression::from(EF::from(value)), PhantomData)
    }
}

impl<F, EF> From<TrackedFieldExpression<F>> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn from(value: TrackedFieldExpression<F>) -> Self {
        Self::from(value.value)
    }
}

impl<F, EF> Add for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, PhantomData)
    }
}

impl<F, EF> Add<TrackedFieldExpression<F>> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    type Output = Self;

    fn add(self, rhs: TrackedFieldExpression<F>) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl<F, EF> AddAssign for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs;
    }
}

impl<F, EF> AddAssign<TrackedFieldExpression<F>> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn add_assign(&mut self, rhs: TrackedFieldExpression<F>) {
        *self = self.clone() + rhs;
    }
}

impl<F, EF> Sum for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x + y).unwrap_or(Self::zero())
    }
}

impl<F, EF> Sub for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0, PhantomData)
    }
}

impl<F, EF> Sub<TrackedFieldExpression<F>> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    type Output = Self;

    fn sub(self, rhs: TrackedFieldExpression<F>) -> Self::Output {
        self - Self::from(rhs)
    }
}

impl<F, EF> SubAssign for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone() - rhs;
    }
}

impl<F, EF> SubAssign<TrackedFieldExpression<F>> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn sub_assign(&mut self, rhs: TrackedFieldExpression<F>) {
        *self = self.clone() - rhs;
    }
}

impl<F, EF> Neg for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0, PhantomData)
    }
}

impl<F, EF> Mul for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0, PhantomData)
    }
}

impl<F, EF> Mul<TrackedFieldExpression<F>> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    type Output = Self;

    fn mul(self, rhs: TrackedFieldExpression<F>) -> Self::Output {
        self * Self::from(rhs)
    }
}

impl<F, EF> MulAssign for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.clone() * rhs;
    }
}

impl<F, EF> MulAssign<TrackedFieldExpression<F>> for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn mul_assign(&mut self, rhs: TrackedFieldExpression<F>) {
        *self = self.clone() * rhs;
    }
}

impl<F, EF> Product for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|x, y| x * y).unwrap_or(Self::one())
    }
}
impl<F, EF> AbstractField for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
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

impl<F, EF> AbstractExtensionField<TrackedFieldExpression<F>>
    for TrackedExtensionFieldExpression<F, EF>
where
    F: AbstractField,
    EF: AbstractExtensionField<F>,
{
    const D: usize = EF::D;

    fn from_base(b: TrackedFieldExpression<F>) -> Self {
        Self::from(b)
    }

    fn from_base_slice(bs: &[TrackedFieldExpression<F>]) -> Self {
        let bs = bs.iter().map(|b| b.value.clone()).collect::<Vec<_>>();
        let value = EF::from_base_slice(&bs);
        Self(TrackedFieldExpression::from(value), PhantomData)
    }

    fn from_base_fn<FN: FnMut(usize) -> TrackedFieldExpression<F>>(mut f: FN) -> Self {
        let value = EF::from_base_fn(|i| f(i).value.clone());
        Self(TrackedFieldExpression::from(value), PhantomData)
    }

    fn as_base_slice(&self) -> &[TrackedFieldExpression<F>] {
        unimplemented!("TrackedExtensionFieldExpression::as_base_slice")
    }
}

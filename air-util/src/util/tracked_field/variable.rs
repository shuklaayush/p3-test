use alloc::collections::BTreeSet;
use alloc::vec;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::ops::{Add, Mul, Sub};

use p3_field::{ExtensionField, Field};

use super::expression::TrackedFieldExpression;
use super::TrackedExtensionFieldExpression;

#[derive(Copy, Clone, Debug, Default)]
pub struct TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    pub value: F,
    pub entry: E,
}

impl<F, E> TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    pub fn new(value: F, entry: E) -> Self {
        Self { value, entry }
    }

    pub fn new_untracked(value: F) -> Self {
        Self {
            value,
            entry: E::default(),
        }
    }
}

impl<F, E> From<TrackedFieldVariable<F, E>> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    fn from(value: TrackedFieldVariable<F, E>) -> Self {
        TrackedFieldExpression {
            value: value.value,
            origin: BTreeSet::from_iter(vec![value.entry]),
        }
    }
}

impl<F, EF, E> From<TrackedFieldVariable<EF, E>> for TrackedExtensionFieldExpression<F, EF, E>
where
    F: Field,
    EF: ExtensionField<F>,
    E: Default + Clone + Debug + Ord,
{
    fn from(value: TrackedFieldVariable<EF, E>) -> Self {
        Self(TrackedFieldExpression::from(value), PhantomData)
    }
}

impl<F, E> Add for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn add(self, rhs: Self) -> Self::Output {
        TrackedFieldExpression::from(self) + TrackedFieldExpression::from(rhs)
    }
}

impl<F, E> Add<F> for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn add(self, rhs: F) -> Self::Output {
        TrackedFieldExpression::from(self) + TrackedFieldExpression::from(rhs)
    }
}

impl<F, E> Add<TrackedFieldExpression<F, E>> for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn add(self, rhs: TrackedFieldExpression<F, E>) -> Self::Output {
        TrackedFieldExpression::from(self) + rhs
    }
}

impl<F, E> Add<TrackedFieldVariable<F, E>> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn add(self, rhs: TrackedFieldVariable<F, E>) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl<F, E> Sub for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn sub(self, rhs: Self) -> Self::Output {
        TrackedFieldExpression::from(self) - TrackedFieldExpression::from(rhs)
    }
}

impl<F, E> Sub<F> for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn sub(self, rhs: F) -> Self::Output {
        TrackedFieldExpression::from(self) - TrackedFieldExpression::from(rhs)
    }
}

impl<F, E> Sub<TrackedFieldExpression<F, E>> for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn sub(self, rhs: TrackedFieldExpression<F, E>) -> Self::Output {
        TrackedFieldExpression::from(self) - rhs
    }
}

impl<F, E> Sub<TrackedFieldVariable<F, E>> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn sub(self, rhs: TrackedFieldVariable<F, E>) -> Self::Output {
        self - Self::from(rhs)
    }
}

impl<F, E> Mul for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn mul(self, rhs: Self) -> Self::Output {
        TrackedFieldExpression::from(self) * TrackedFieldExpression::from(rhs)
    }
}

impl<F, E> Mul<F> for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn mul(self, rhs: F) -> Self::Output {
        TrackedFieldExpression::from(self) * TrackedFieldExpression::from(rhs)
    }
}

impl<F, E> Mul<TrackedFieldExpression<F, E>> for TrackedFieldVariable<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = TrackedFieldExpression<F, E>;

    fn mul(self, rhs: TrackedFieldExpression<F, E>) -> Self::Output {
        TrackedFieldExpression::from(self) * rhs
    }
}

impl<F, E> Mul<TrackedFieldVariable<F, E>> for TrackedFieldExpression<F, E>
where
    F: Field,
    E: Default + Clone + Debug + Ord,
{
    type Output = Self;

    fn mul(self, rhs: TrackedFieldVariable<F, E>) -> Self::Output {
        self * Self::from(rhs)
    }
}

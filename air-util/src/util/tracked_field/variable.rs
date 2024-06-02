use core::fmt::Debug;
use core::hash::Hash;
use core::ops::{Add, Mul, Sub};
use std::collections::BTreeSet;
use std::marker::PhantomData;

use p3_field::{ExtensionField, Field};

use super::expression::TrackedFieldExpression;
use super::TrackedExtensionFieldExpression;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Entry {
    None,
    Preprocessed { row: usize, col: usize },
    Main { row: usize, col: usize },
    Permutation { row: usize, col: usize },
    Public { index: usize },
}

impl Default for Entry {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TrackedFieldVariable<F: Field> {
    pub value: F,
    pub entry: Entry,
}

impl<F: Field> TrackedFieldVariable<F> {
    pub const fn new(value: F, entry: Entry) -> Self {
        Self { value, entry }
    }

    pub const fn new_untracked(value: F) -> Self {
        Self {
            value,
            entry: Entry::None,
        }
    }
}

impl<F: Field> From<TrackedFieldVariable<F>> for TrackedFieldExpression<F> {
    fn from(value: TrackedFieldVariable<F>) -> Self {
        TrackedFieldExpression {
            value: value.value,
            origin: BTreeSet::from_iter(vec![value.entry]),
        }
    }
}

impl<F, EF> From<TrackedFieldVariable<EF>> for TrackedExtensionFieldExpression<F, EF>
where
    F: Field,
    EF: ExtensionField<F>,
{
    fn from(value: TrackedFieldVariable<EF>) -> Self {
        Self(TrackedFieldExpression::from(value), PhantomData)
    }
}

impl<F: Field> Add for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn add(self, rhs: Self) -> Self::Output {
        TrackedFieldExpression::from(self) + TrackedFieldExpression::from(rhs)
    }
}

impl<F: Field> Add<F> for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn add(self, rhs: F) -> Self::Output {
        TrackedFieldExpression::from(self) + TrackedFieldExpression::from(rhs)
    }
}

impl<F: Field> Add<TrackedFieldExpression<F>> for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn add(self, rhs: TrackedFieldExpression<F>) -> Self::Output {
        TrackedFieldExpression::from(self) + rhs
    }
}

impl<F: Field> Add<TrackedFieldVariable<F>> for TrackedFieldExpression<F> {
    type Output = Self;

    fn add(self, rhs: TrackedFieldVariable<F>) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl<F: Field> Sub for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn sub(self, rhs: Self) -> Self::Output {
        TrackedFieldExpression::from(self) - TrackedFieldExpression::from(rhs)
    }
}

impl<F: Field> Sub<F> for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn sub(self, rhs: F) -> Self::Output {
        TrackedFieldExpression::from(self) - TrackedFieldExpression::from(rhs)
    }
}

impl<F: Field> Sub<TrackedFieldExpression<F>> for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn sub(self, rhs: TrackedFieldExpression<F>) -> Self::Output {
        TrackedFieldExpression::from(self) - rhs
    }
}

impl<F: Field> Sub<TrackedFieldVariable<F>> for TrackedFieldExpression<F> {
    type Output = Self;

    fn sub(self, rhs: TrackedFieldVariable<F>) -> Self::Output {
        self - Self::from(rhs)
    }
}

impl<F: Field> Mul for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn mul(self, rhs: Self) -> Self::Output {
        TrackedFieldExpression::from(self) * TrackedFieldExpression::from(rhs)
    }
}

impl<F: Field> Mul<F> for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn mul(self, rhs: F) -> Self::Output {
        TrackedFieldExpression::from(self) * TrackedFieldExpression::from(rhs)
    }
}

impl<F: Field> Mul<TrackedFieldExpression<F>> for TrackedFieldVariable<F> {
    type Output = TrackedFieldExpression<F>;

    fn mul(self, rhs: TrackedFieldExpression<F>) -> Self::Output {
        TrackedFieldExpression::from(self) * rhs
    }
}

impl<F: Field> Mul<TrackedFieldVariable<F>> for TrackedFieldExpression<F> {
    type Output = Self;

    fn mul(self, rhs: TrackedFieldVariable<F>) -> Self::Output {
        self * Self::from(rhs)
    }
}

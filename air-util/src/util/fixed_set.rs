use core::fmt::Debug;
use std::hash::Hash;
use std::ops::{BitAnd, BitOr, BitXor, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FixedSet<const SIZE: usize> {
    elements: [Option<usize>; SIZE],
    size: usize,
}

impl<const SIZE: usize> Default for FixedSet<SIZE> {
    fn default() -> Self {
        FixedSet {
            elements: [None; SIZE],
            size: 0,
        }
    }
}

impl<const SIZE: usize> FixedSet<SIZE> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.elements.iter().filter_map(|&x| x)
    }

    pub fn insert(&mut self, value: usize) -> bool {
        if self.contains(value) {
            return false;
        }
        assert!(self.size < SIZE, "FixedSet is full");

        self.elements[self.size] = Some(value);
        self.size += 1;
        true
    }

    pub fn contains(&self, value: usize) -> bool {
        self.elements.iter().any(|&x| x == Some(value))
    }
}

impl<const SIZE: usize> BitOr for FixedSet<SIZE> {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        if self.size == 0 {
            other
        } else if other.size == 0 {
            self
        } else {
            let mut result = FixedSet::default();
            for &elem in self.elements.iter().chain(other.elements.iter()) {
                if let Some(value) = elem {
                    result.insert(value);
                }
            }
            result
        }
    }
}

impl<const SIZE: usize> BitAnd for FixedSet<SIZE> {
    type Output = Self;

    fn bitand(self, other: Self) -> Self::Output {
        if self.size == 0 || other.size == 0 {
            FixedSet::default()
        } else {
            let mut result = FixedSet::default();
            for &elem in self.elements.iter() {
                if let Some(value) = elem {
                    if other.contains(value) {
                        result.insert(value);
                    }
                }
            }
            result
        }
    }
}

impl<const SIZE: usize> Sub for FixedSet<SIZE> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        if self.size == 0 || other.size == 0 {
            self
        } else {
            let mut result = FixedSet::default();
            for &elem in self.elements.iter() {
                if let Some(value) = elem {
                    if !other.contains(value) {
                        result.insert(value);
                    }
                }
            }
            result
        }
    }
}

impl<const SIZE: usize> BitXor for FixedSet<SIZE> {
    type Output = Self;

    fn bitxor(self, other: Self) -> Self::Output {
        if self.size == 0 {
            other
        } else if other.size == 0 {
            self
        } else {
            let mut result = FixedSet::default();
            for &elem in self.elements.iter() {
                if let Some(value) = elem {
                    if !other.contains(value) {
                        result.insert(value);
                    }
                }
            }
            for &elem in other.elements.iter() {
                if let Some(value) = elem {
                    if !self.contains(value) {
                        result.insert(value);
                    }
                }
            }
            result
        }
    }
}

use core::fmt::Debug;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceEntry {
    None,
    Preprocessed { row: usize, col: usize },
    Main { row: usize, col: usize },
    Permutation { row: usize, col: usize },
    Public { index: usize },
}

impl Default for TraceEntry {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MultiTraceEntry {
    None,
    Preprocessed {
        trace: usize,
        row: usize,
        col: usize,
    },
    Main {
        trace: usize,
        row: usize,
        col: usize,
    },
    Permutation {
        trace: usize,
        row: usize,
        col: usize,
    },
    Public {
        index: usize,
    },
}

impl Default for MultiTraceEntry {
    fn default() -> Self {
        Self::None
    }
}

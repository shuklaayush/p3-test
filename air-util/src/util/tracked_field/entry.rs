use core::fmt::Debug;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnEntry {
    None,
    Preprocessed { col: usize },
    Main { col: usize },
    Permutation { col: usize },
    VirtualColumnCount { interaction: usize },
    VirtualColumnField { interaction: usize, field: usize },
    Public { index: usize },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceEntry {
    None,
    Preprocessed {
        row: usize,
        col: usize,
    },
    Main {
        row: usize,
        col: usize,
    },
    Permutation {
        row: usize,
        col: usize,
    },
    VirtualColumnCount {
        row: usize,
        interaction: usize,
    },
    VirtualColumnField {
        row: usize,
        interaction: usize,
        field: usize,
    },
    Public {
        index: usize,
    },
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
    VirtualColumnCount {
        trace: usize,
        row: usize,
        interaction: usize,
    },
    VirtualColumnField {
        trace: usize,
        row: usize,
        interaction: usize,
        field: usize,
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

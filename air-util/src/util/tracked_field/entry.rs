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

impl From<MultiTraceEntry> for TraceEntry {
    fn from(entry: MultiTraceEntry) -> Self {
        match entry {
            MultiTraceEntry::None => TraceEntry::None,
            MultiTraceEntry::Preprocessed { row, col, .. } => TraceEntry::Preprocessed { row, col },
            MultiTraceEntry::Main { row, col, .. } => TraceEntry::Main { row, col },
            MultiTraceEntry::Permutation { row, col, .. } => TraceEntry::Permutation { row, col },
            MultiTraceEntry::VirtualColumnCount {
                row, interaction, ..
            } => TraceEntry::VirtualColumnCount { row, interaction },
            MultiTraceEntry::VirtualColumnField {
                row,
                interaction,
                field,
                ..
            } => TraceEntry::VirtualColumnField {
                row,
                interaction,
                field,
            },
            MultiTraceEntry::Public { index } => TraceEntry::Public { index },
        }
    }
}

impl From<TraceEntry> for ColumnEntry {
    fn from(entry: TraceEntry) -> Self {
        match entry {
            TraceEntry::None => ColumnEntry::None,
            TraceEntry::Preprocessed { col, .. } => ColumnEntry::Preprocessed { col },
            TraceEntry::Main { col, .. } => ColumnEntry::Main { col },
            TraceEntry::Permutation { col, .. } => ColumnEntry::Permutation { col },
            TraceEntry::VirtualColumnCount { interaction, .. } => {
                ColumnEntry::VirtualColumnCount { interaction }
            }
            TraceEntry::VirtualColumnField {
                interaction, field, ..
            } => ColumnEntry::VirtualColumnField { interaction, field },
            TraceEntry::Public { index } => ColumnEntry::Public { index },
        }
    }
}

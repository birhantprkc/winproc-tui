use super::{CpuCoreKind, scheduling::PriorityOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AffinityChange {
    pub(crate) expected: usize,
    pub(crate) desired: usize,
    pub(crate) restore: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AffinityReport {
    pub(crate) current: Option<usize>,
    pub(crate) allowed: usize,
    pub(crate) kinds: Vec<Option<CpuCoreKind>>,
    pub(crate) restore: Option<AffinityChange>,
    pub(crate) outcome: PriorityOutcome,
}
impl AffinityReport {
    pub(crate) fn unavailable(error: String) -> Self {
        Self {
            current: None,
            allowed: 0,
            kinds: Vec::new(),
            restore: None,
            outcome: PriorityOutcome::Failed(error),
        }
    }
    pub(crate) fn cpus(&self) -> Vec<usize> {
        (0..usize::BITS as usize)
            .filter(|bit| self.allowed & (1usize << bit) != 0)
            .collect()
    }
}

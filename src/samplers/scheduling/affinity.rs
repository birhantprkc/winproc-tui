use super::{NativeProcess, native_error};
use crate::model::{
    affinity::{AffinityChange, AffinityReport},
    scheduling::PriorityOutcome,
};
use winapi::um::winbase::{GetActiveProcessorGroupCount, GetProcessAffinityMask};

// winapi 0.3.9 incorrectly declares this mask as DWORD (32 bits). The Windows
// API takes DWORD_PTR; keep all 64 logical CPU bits on Windows x64.
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetProcessAffinityMask(process: winapi::shared::ntdef::HANDLE, mask: usize) -> i32;
}

pub(super) trait AffinityProcess {
    fn read_affinity(&mut self) -> Result<(usize, usize), String>;
    fn set_affinity(&mut self, mask: usize) -> Result<(), String>;
}

pub(super) fn validate(groups: u16, current: usize, allowed: usize) -> Result<(), String> {
    if groups > 1 {
        return Err("CPU affinity unsupported: multiple processor groups.".into());
    }
    if groups != 1 || current == 0 || allowed == 0 || current & !allowed != 0 {
        return Err("CPU affinity unavailable: group or CPU mask could not be verified.".into());
    }
    Ok(())
}

pub(super) fn execute<P: AffinityProcess>(
    process: &mut P,
    writable: bool,
    restore: &mut Option<AffinityChange>,
    change: Option<AffinityChange>,
    active: impl Fn() -> bool,
) -> AffinityReport {
    let (current, allowed) = match process.read_affinity() {
        Ok(value) => value,
        Err(error) => {
            let mut report = AffinityReport::unavailable(error);
            report.restore = *restore;
            return report;
        }
    };
    let mut report = AffinityReport {
        current: Some(current),
        allowed,
        kinds: Vec::new(),
        restore: *restore,
        outcome: PriorityOutcome::Read,
    };
    let Some(change) = change else {
        return report;
    };
    let error = if !writable {
        Some("Read-only: permission to change affinity is unavailable.")
    } else if change.desired == 0 || change.desired & !allowed != 0 {
        Some("Select at least one allowed CPU; the mask must stay within the system mask.")
    } else if current != change.expected {
        Some("Affinity changed externally; refresh and review the current value.")
    } else if change.restore && *restore != Some(change) {
        Some("The previous affinity is no longer eligible for restoration.")
    } else if !active() {
        Some("Affinity change cancelled before application.")
    } else {
        None
    };
    if let Some(error) = error {
        report.outcome = PriorityOutcome::Failed(error.into());
        return report;
    }
    if let Err(error) = process.set_affinity(change.desired) {
        report.outcome = PriorityOutcome::Failed(error);
        return report;
    }
    *restore = if change.restore {
        None
    } else {
        Some(AffinityChange {
            expected: change.desired,
            desired: current,
            restore: true,
        })
    };
    report.restore = *restore;
    match process.read_affinity() {
        Ok((actual, allowed)) => {
            report.current = Some(actual);
            report.allowed = allowed;
            report.outcome = if actual != change.desired {
                PriorityOutcome::AppliedUnverified(format!(
                    "Change accepted; readback is 0x{actual:X}, expected 0x{:X}. Refresh before continuing.",
                    change.desired
                ))
            } else if change.restore {
                PriorityOutcome::Restored
            } else {
                PriorityOutcome::Changed
            };
        }
        Err(error) => {
            report.current = None;
            report.outcome = PriorityOutcome::AppliedUnverified(format!(
                "Affinity change accepted; verification failed: {error}"
            ));
        }
    }
    report
}

impl AffinityProcess for NativeProcess {
    fn read_affinity(&mut self) -> Result<(usize, usize), String> {
        if self.live_creation_time()? != self.creation_time {
            return Err("Process identity changed".into());
        }
        // SAFETY: group count has no pointer arguments; the retained handle has query access,
        // and both mask outputs refer to distinct writable usize values.
        let groups = unsafe { GetActiveProcessorGroupCount() };
        if groups != 1 {
            validate(groups, 0, 0)?;
        }
        let (mut current, mut allowed) = (0, 0);
        if unsafe { GetProcessAffinityMask(self.handle, &mut current, &mut allowed) } == 0 {
            return Err(native_error("GetProcessAffinityMask"));
        }
        validate(groups, current, allowed)?;
        Ok((current, allowed))
    }
    fn set_affinity(&mut self, mask: usize) -> Result<(), String> {
        let (_, allowed) = self.read_affinity()?;
        if mask == 0 || mask & !allowed != 0 {
            return Err("Invalid affinity mask".into());
        }
        // SAFETY: the session holds the verified process object with set access and a
        // nonempty mask restricted to a freshly verified single-group system mask.
        if unsafe { SetProcessAffinityMask(self.handle, mask) } == 0 {
            Err(native_error("SetProcessAffinityMask"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        current: usize,
        allowed: usize,
        groups: u16,
        writes: usize,
        fail_set: bool,
        fail_readback: bool,
        changed_readback: bool,
        exited: bool,
    }
    impl AffinityProcess for Fake {
        fn read_affinity(&mut self) -> Result<(usize, usize), String> {
            if self.exited || (self.fail_readback && self.writes > 0) {
                return Err("Process unavailable".into());
            }
            validate(self.groups, self.current, self.allowed)?;
            Ok((
                if self.changed_readback && self.writes > 0 {
                    self.allowed
                } else {
                    self.current
                },
                self.allowed,
            ))
        }
        fn set_affinity(&mut self, mask: usize) -> Result<(), String> {
            if self.fail_set {
                return Err("Access denied".into());
            }
            self.writes += 1;
            self.current = mask;
            Ok(())
        }
    }
    fn fake() -> Fake {
        Fake {
            current: 0b1011,
            allowed: 0b1011,
            groups: 1,
            writes: 0,
            fail_set: false,
            fail_readback: false,
            changed_readback: false,
            exited: false,
        }
    }
    fn change(desired: usize) -> AffinityChange {
        AffinityChange {
            expected: 0b1011,
            desired,
            restore: false,
        }
    }

    #[test]
    fn affinity_one_multiple_all_cpus_and_explicit_restore() {
        for mask in [1, 3, 0b1011] {
            let mut p = fake();
            let mut restore = None;
            let report = execute(&mut p, true, &mut restore, Some(change(mask)), || true);
            assert_eq!(report.current, Some(mask));
            assert_eq!(report.outcome, PriorityOutcome::Changed);
            let request = restore;
            let report = execute(&mut p, true, &mut restore, request, || true);
            assert_eq!(report.current, Some(0b1011));
            assert_eq!(report.outcome, PriorityOutcome::Restored);
            assert!(restore.is_none());
        }
    }
    #[test]
    fn affinity_rejects_invalid_empty_readonly_external_cancel_exit_unknown_and_multiple_groups() {
        for mask in [0, 4, usize::MAX] {
            let mut p = fake();
            let mut restore = None;
            assert!(matches!(
                execute(&mut p, true, &mut restore, Some(change(mask)), || true).outcome,
                PriorityOutcome::Failed(_)
            ));
            assert_eq!(p.writes, 0);
        }
        for case in 0..8 {
            let mut p = fake();
            let mut restore = None;
            match case {
                2 => p.current = 3,
                3 => p.exited = true,
                4 => p.groups = 2,
                5 => p.groups = 0,
                6 => p.allowed = 0,
                7 => p.fail_set = true,
                _ => {}
            }
            assert!(matches!(
                execute(&mut p, case != 0, &mut restore, Some(change(1)), || case
                    != 1)
                .outcome,
                PriorityOutcome::Failed(_)
            ));
            assert_eq!(p.writes, 0);
            assert!(restore.is_none());
        }
    }
    #[test]
    fn affinity_readback_failure_and_external_change_never_silently_restore() {
        for mismatch in [false, true] {
            let mut p = fake();
            p.fail_readback = !mismatch;
            p.changed_readback = mismatch;
            let mut restore = None;
            let report = execute(&mut p, true, &mut restore, Some(change(1)), || true);
            assert!(matches!(
                report.outcome,
                PriorityOutcome::AppliedUnverified(_)
            ));
            p.fail_readback = false;
            p.changed_readback = false;
            p.current = 3;
            let request = restore;
            assert!(matches!(
                execute(&mut p, true, &mut restore, request, || true).outcome,
                PriorityOutcome::Failed(_)
            ));
            assert_eq!(p.writes, 1);
            p.current = 1;
            let request = restore;
            assert_eq!(
                execute(&mut p, true, &mut restore, request, || true).outcome,
                PriorityOutcome::Restored
            );
        }
    }
}

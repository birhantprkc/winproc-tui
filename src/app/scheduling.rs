use super::{App, AppActivity, ProcessInfoFocus};
use crate::{
    model::scheduling::{PriorityChange, PriorityClass, PriorityOutcome, PriorityReport},
    samplers::scheduling::SchedulingRequest,
    ui::widgets::scrollable_modal::ScrollableModalState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::sync::mpsc::TryRecvError;

pub(crate) struct SchedulingView {
    pub(crate) report: Option<PriorityReport>,
    pub(crate) pending: bool,
    pub(crate) applying: bool,
    pub(crate) selected: usize,
    pub(crate) confirmation: Option<PriorityChange>,
    pub(crate) notice: Option<String>,
    pub(crate) scroll: ScrollableModalState,
}

impl Default for SchedulingView {
    fn default() -> Self {
        Self {
            report: None,
            pending: false,
            applying: false,
            selected: 2,
            confirmation: None,
            notice: None,
            scroll: ScrollableModalState::default(),
        }
    }
}

impl App {
    pub(crate) fn reset_scheduling(&mut self) {
        self.scheduling_worker.cancel();
        self.scheduling = SchedulingView::default();
    }

    pub(crate) fn ensure_scheduling(&mut self) {
        if self.scheduling.report.is_none() && !self.scheduling.pending {
            self.refresh_scheduling();
        }
    }

    pub(crate) fn scheduling_disabled_reason(&self) -> Option<&'static str> {
        if self.activity() == AppActivity::LogView {
            Some("Not recorded in Log view.")
        } else if self.is_display_paused() {
            Some("Resume the display before changing priority.")
        } else if !self.show_process_info_dialog || !self.process_info_target_is_currently_live() {
            Some("Process exited or no live process is selected.")
        } else if self
            .scheduling
            .report
            .as_ref()
            .and_then(|report| report.current)
            .is_none()
        {
            Some("Current priority is unavailable; refresh to retry.")
        } else if !self
            .scheduling
            .report
            .as_ref()
            .is_some_and(|report| report.writable)
        {
            Some("Read-only: changing priority is unavailable.")
        } else {
            None
        }
    }

    pub(crate) fn refresh_scheduling(&mut self) {
        if self.activity() == AppActivity::LogView
            || !self.show_process_info_dialog
            || self.scheduling.pending
        {
            return;
        }
        if !self.process_info_target_is_currently_live() {
            self.scheduling.notice = Some("Process exited or no live process is selected.".into());
            return;
        }
        self.scheduling.confirmation = None;
        self.submit_scheduling(None);
    }

    fn submit_scheduling(&mut self, change: Option<PriorityChange>) {
        let Some(target) = self.process_info_target.as_ref() else {
            return;
        };
        match self.scheduling_worker.request(SchedulingRequest {
            generation: self.process_info_generation,
            identity: target.identity.clone(),
            change,
        }) {
            Ok(()) => {
                self.scheduling.pending = true;
                self.scheduling.applying = change.is_some();
                self.scheduling.notice = None;
            }
            Err(error) => self.scheduling.notice = Some(error),
        }
    }

    pub(crate) fn poll_scheduling_results(&mut self) -> bool {
        let mut changed = false;
        loop {
            match self.scheduling_worker.try_recv() {
                Ok(result) => {
                    if !self.show_process_info_dialog
                        || self.activity() == AppActivity::LogView
                        || !self.scheduling.pending
                        || result.generation != self.process_info_generation
                        || self
                            .process_info_target
                            .as_ref()
                            .map(|target| &target.identity)
                            != Some(&result.identity)
                    {
                        continue;
                    }
                    if self.scheduling.report.is_none()
                        || matches!(
                            result.report.outcome,
                            PriorityOutcome::Changed | PriorityOutcome::Restored
                        )
                    {
                        self.scheduling.selected = result
                            .report
                            .current
                            .and_then(|current| {
                                PriorityClass::EDITABLE
                                    .iter()
                                    .position(|class| *class == current)
                            })
                            .unwrap_or(2);
                    }
                    self.scheduling.report = Some(result.report);
                    self.scheduling.pending = false;
                    self.scheduling.applying = false;
                    self.scheduling.confirmation = None;
                    changed = true;
                }
                Err(TryRecvError::Empty) => return changed,
                Err(TryRecvError::Disconnected) => {
                    if self.scheduling.pending {
                        self.scheduling.pending = false;
                        self.scheduling.applying = false;
                        self.scheduling.notice = Some(
                            "Scheduling worker stopped; the result of a pending change is unknown."
                                .into(),
                        );
                        return true;
                    }
                    return changed;
                }
            }
        }
    }

    pub(crate) fn prepare_priority_change(&mut self, restore: bool) {
        if self.scheduling.pending {
            return;
        }
        if let Some(reason) = self.scheduling_disabled_reason() {
            self.scheduling.notice = Some(reason.into());
            return;
        }
        let Some(report) = self.scheduling.report.as_ref() else {
            return;
        };
        let Some(current) = report.current else {
            self.scheduling.notice = Some("Refresh to verify the current priority first.".into());
            return;
        };
        let change = if restore {
            let Some(change) = report.restore else {
                self.scheduling.notice =
                    Some("No previous change to restore in this Process Info session.".into());
                return;
            };
            if change.expected != current {
                self.scheduling.notice =
                    Some("Priority changed externally; restoration is unavailable.".into());
                return;
            }
            change
        } else {
            PriorityChange {
                expected: current,
                desired: PriorityClass::EDITABLE[self.scheduling.selected],
                restore: false,
            }
        };
        if !change.desired.editable() {
            self.scheduling.notice = Some("The previous priority cannot be restored: Realtime and unknown classes are not selectable.".into());
            return;
        }
        if change.expected == change.desired {
            self.scheduling.notice = Some("The selected priority is already current.".into());
            return;
        }
        self.scheduling.confirmation = Some(change);
        self.scheduling.notice = None;
        self.scheduling.scroll.scroll_home();
    }

    fn confirm_priority_change(&mut self) {
        if self.scheduling.pending {
            return;
        }
        if let Some(reason) = self.scheduling_disabled_reason() {
            self.scheduling.confirmation = None;
            self.scheduling.notice = Some(reason.into());
            return;
        }
        if let Some(change) = self.scheduling.confirmation.take() {
            self.submit_scheduling(Some(change));
        }
    }

    pub(crate) fn on_scheduling_key(&mut self, key: KeyEvent) {
        if self.scheduling.applying {
            return;
        }
        if self.scheduling.confirmation.is_some() {
            match key.code {
                KeyCode::Enter => self.confirm_priority_change(),
                KeyCode::Esc => self.scheduling.confirmation = None,
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Esc => self.close_process_info_dialog(),
            KeyCode::Enter => self.prepare_priority_change(false),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.refresh_scheduling()
            }
            KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.prepare_priority_change(true)
            }
            KeyCode::Up => self.select_priority(self.scheduling.selected.saturating_sub(1)),
            KeyCode::Down => self.select_priority((self.scheduling.selected + 1).min(4)),
            KeyCode::Home => self.select_priority(0),
            KeyCode::End => self.select_priority(4),
            KeyCode::PageUp => self
                .scheduling
                .scroll
                .scroll_up(self.scheduling.scroll.page_size),
            KeyCode::PageDown => {
                let total =
                    crate::ui::scheduling::lines(self, self.last_screen_area.width, self.theme())
                        .len();
                self.scheduling
                    .scroll
                    .scroll_down(self.scheduling.scroll.page_size, total);
            }
            _ => {}
        }
    }

    pub(crate) fn select_priority(&mut self, index: usize) {
        self.scheduling.selected = index.min(4);
        let area = crate::ui::process_info_content_area_for_screen(self.last_screen_area);
        let total = crate::ui::scheduling::lines(self, area.width, self.theme()).len();
        self.scheduling.scroll.ensure_visible(
            crate::ui::scheduling::OPTION_START + self.scheduling.selected,
            total,
        );
    }

    pub(crate) fn scheduling_mouse_selection(&mut self, mouse: MouseEvent, screen: Rect) -> bool {
        if self.scheduling.confirmation.is_some() || self.scheduling.applying {
            return true;
        }
        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            let area = crate::ui::process_info_content_area_for_screen(screen);
            if let Some(index) =
                crate::ui::scheduling::option_at(self, area, mouse.column, mouse.row)
            {
                self.process_info_focus = ProcessInfoFocus::Content;
                self.select_priority(index);
                return true;
            }
        }
        false
    }
}

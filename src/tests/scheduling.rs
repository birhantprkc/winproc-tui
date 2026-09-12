use super::support::{find_text_position, make_test_app, render_app_to_buffer, render_app_to_text};
use crate::{
    app::{App, ProcessInfoFocus, ProcessInfoTab},
    model::scheduling::{PriorityChange, PriorityClass, PriorityOutcome, PriorityReport},
    samplers::scheduling::{SchedulingRequest, SchedulingResult, SchedulingWorker},
    ui,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{layout::Rect, prelude::Modifier};
use std::sync::mpsc::{Receiver, Sender};

struct Fixture {
    app: App,
    requests: Receiver<SchedulingRequest>,
    results: Sender<SchedulingResult>,
}
impl Fixture {
    fn new() -> Self {
        let mut app = make_test_app(2, 10);
        let (worker, requests, results) = SchedulingWorker::test_pair();
        app.scheduling_worker = worker;
        app.open_selected_process_info_dialog().unwrap();
        app.activate_process_info_tab(ProcessInfoTab::Scheduling)
            .unwrap();
        app.process_info_focus = ProcessInfoFocus::Content;
        Self {
            app,
            requests,
            results,
        }
    }
    fn key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.app.on_key(KeyEvent::new(code, modifiers)).unwrap();
    }
    fn deliver(&mut self, request: SchedulingRequest, report: PriorityReport) -> bool {
        self.results
            .send(SchedulingResult {
                generation: request.generation,
                identity: request.identity,
                report,
            })
            .unwrap();
        self.app.poll_scheduling_results()
    }
    fn ready(&mut self) {
        let request = self.requests.try_recv().unwrap();
        assert!(request.change.is_none());
        assert!(self.deliver(request, report(PriorityClass::Normal)));
    }
}
fn report(current: PriorityClass) -> PriorityReport {
    PriorityReport {
        current: Some(current),
        writable: true,
        restore: None,
        outcome: PriorityOutcome::Read,
    }
}

#[test]
fn scheduling_requires_review_and_confirmation_for_change_and_restore_on_the_fixed_target() {
    let mut f = Fixture::new();
    f.ready();
    f.app.process_table_state.select(Some(1));
    f.key(KeyCode::Down, KeyModifiers::NONE);
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    let change = f.app.scheduling.confirmation.unwrap();
    assert_eq!(
        change,
        PriorityChange {
            expected: PriorityClass::Normal,
            desired: PriorityClass::AboveNormal,
            restore: false
        }
    );
    assert!(f.requests.try_recv().is_err());
    f.key(KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(f.app.process_info_tab, ProcessInfoTab::Scheduling);
    let buffer = render_app_to_buffer(&f.app, 100, 30);
    let text = super::support::buffer_to_text(&buffer);
    for expected in [
        "Current:  Normal",
        "Proposed: Above normal",
        "Enter apply",
        "Esc cancel",
        "proc-0",
    ] {
        assert!(text.contains(expected), "{text}");
    }
    for key in ["Enter", "Esc"] {
        let (x, y) = find_text_position(&buffer, key).unwrap();
        let cell = &buffer[(x, y)];
        assert_eq!(cell.fg, f.app.theme().warning);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }
    f.key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(f.app.scheduling.confirmation.is_none());
    assert!(f.requests.try_recv().is_err());
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    let request = f.requests.try_recv().unwrap();
    assert_eq!(request.identity.name, "proc-0");
    assert_eq!(request.change, Some(change));
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    f.key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(f.app.show_process_info_dialog);
    assert!(f.requests.try_recv().is_err());
    let restore = PriorityChange {
        expected: change.desired,
        desired: change.expected,
        restore: true,
    };
    let mut changed = report(change.desired);
    changed.outcome = PriorityOutcome::Changed;
    changed.restore = Some(restore);
    assert!(f.deliver(request, changed));
    f.key(KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert_eq!(f.app.scheduling.confirmation, Some(restore));
    assert!(f.requests.try_recv().is_err());
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(f.requests.try_recv().unwrap().change, Some(restore));
}

#[test]
fn scheduling_blocks_readonly_pause_log_exit_and_realtime_restore() {
    let mut f = Fixture::new();
    f.ready();
    f.app.scheduling.report.as_mut().unwrap().writable = false;
    f.app.prepare_priority_change(false);
    assert!(f.app.scheduling.confirmation.is_none());
    f.app.scheduling.report = Some(report(PriorityClass::Realtime));
    f.app.prepare_priority_change(false);
    assert!(render_app_to_text(&f.app, 160, 35).contains("cannot be restored by this editor"));
    f.key(KeyCode::Esc, KeyModifiers::NONE);
    let mut current = report(PriorityClass::Normal);
    current.restore = Some(PriorityChange {
        expected: PriorityClass::Normal,
        desired: PriorityClass::Realtime,
        restore: true,
    });
    f.app.scheduling.report = Some(current);
    f.app.prepare_priority_change(true);
    assert!(f.app.scheduling.confirmation.is_none());
    f.app.select_priority(4);
    f.app.prepare_priority_change(false);
    f.app.toggle_display_pause();
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(f.app.scheduling.confirmation.is_none());
    f.app.prepare_priority_change(false);
    assert!(f.app.scheduling.confirmation.is_none());
    f.app.toggle_display_pause();
    f.app.log_view_path = Some("recording.log".into());
    f.app.refresh_scheduling();
    f.app.prepare_priority_change(false);
    assert!(render_app_to_text(&f.app, 80, 24).contains("Not recorded in Log view."));
    f.app.log_view_path = None;
    f.app.snapshot.processes.remove(0);
    f.app.refresh_scheduling();
    f.app.prepare_priority_change(false);
    assert!(f.app.scheduling.confirmation.is_none());
    assert!(f.requests.try_recv().is_err());
}

#[test]
fn scheduling_ignores_old_sessions_and_refreshes_only_on_explicit_request() {
    let mut f = Fixture::new();
    let old = f.requests.try_recv().unwrap();
    f.app.close_process_info_dialog();
    f.app.open_selected_process_info_dialog().unwrap();
    let new = f.requests.try_recv().unwrap();
    assert_ne!(old.generation, new.generation);
    assert!(!f.deliver(old, report(PriorityClass::High)));
    assert!(f.app.scheduling.pending);
    assert!(f.deliver(new, report(PriorityClass::Normal)));
    f.app
        .activate_process_info_tab(ProcessInfoTab::Metrics)
        .unwrap();
    f.app
        .activate_process_info_tab(ProcessInfoTab::Scheduling)
        .unwrap();
    assert!(f.requests.try_recv().is_err());
    f.app.refresh_scheduling();
    f.app.refresh_scheduling();
    let refresh = f.requests.try_recv().unwrap();
    assert!(f.requests.try_recv().is_err());
    assert!(f.deliver(refresh, report(PriorityClass::BelowNormal)));
    assert_eq!(
        f.app.scheduling.report.as_ref().unwrap().current,
        Some(PriorityClass::BelowNormal)
    );
}

#[test]
fn scheduling_mouse_selection_and_small_layout_share_option_geometry() {
    let mut f = Fixture::new();
    f.ready();
    for (width, height) in [(60, 18), (80, 24), (160, 35)] {
        let screen = Rect::new(0, 0, width, height);
        crate::app::sync_layout_state(&mut f.app, screen);
        f.app.select_priority(4);
        let area = ui::process_info_content_area_for_screen(screen);
        let row =
            area.y + (ui::scheduling::OPTION_START + 4 - f.app.scheduling.scroll.offset) as u16;
        assert!(row < area.bottom());
        assert_eq!(
            ui::scheduling::option_at(&f.app, area, area.x + 2, row),
            Some(4)
        );
        f.app.scheduling.selected = 0;
        f.app.on_mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: area.x + 2,
                row,
                modifiers: KeyModifiers::NONE,
            },
            screen,
        );
        assert_eq!(f.app.scheduling.selected, 4);
        let rendered = render_app_to_text(&f.app, width, height);
        assert!(rendered.contains("High"), "{rendered}");
        assert!(!rendered.contains("( ) Realtime"));
    }
}

#[test]
fn scheduling_changes_during_recording_do_not_record_priority_or_actions() {
    let mut f = Fixture::new();
    f.ready();
    let path = super::support::unique_recording_path("scheduling");
    super::support::track_process_name(&mut f.app, "proc-0");
    f.app.recording_path_draft = path.to_string_lossy().into_owned();
    f.app.recording_path_cursor = f.app.recording_path_draft.len();
    f.app.show_recording_path_dialog = true;
    f.app.confirm_recording_path().unwrap();
    f.app.select_priority(0);
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    f.key(KeyCode::Enter, KeyModifiers::NONE);
    let request = f.requests.try_recv().unwrap();
    assert!(request.change.is_some());
    let mut changed = report(PriorityClass::Idle);
    changed.outcome = PriorityOutcome::Changed;
    assert!(f.deliver(request, changed));
    f.app.stop_recording().unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    for absent in ["scheduling", "priority", "Priority", "Idle"] {
        assert!(!saved.contains(absent), "{absent}");
    }
    std::fs::remove_file(path).unwrap();
}

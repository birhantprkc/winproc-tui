use super::support::{
    assert_blank_row_above_text, find_text_position, make_test_app, render_app_to_buffer,
    render_app_to_text, track_process_name, unique_recording_path,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;

#[test]
fn quit_key_opens_confirmation_before_exiting() {
    let mut app = make_test_app(1, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();

    assert!(app.show_quit_confirmation);
    assert!(!app.should_quit);

    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();

    assert!(!app.show_quit_confirmation);
    assert!(!app.should_quit);
}

#[test]
fn quit_confirmation_dialog_uses_footer_style_key_help() {
    let mut app = make_test_app(1, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();

    let rendered = render_app_to_text(&app, 100, 45);
    assert!(rendered.contains("Quit winproc-tui?"), "{rendered}");
    assert!(
        !rendered.contains("Close winproc-tui and return to terminal."),
        "{rendered}"
    );
    assert!(!rendered.contains("[ Quit ]"), "{rendered}");
    assert!(!rendered.contains("[ Cancel ]"), "{rendered}");
    assert!(rendered.contains("Enter/q Quit  Esc Cancel"), "{rendered}");
    assert!(
        !rendered.contains("Confirm before closing the monitor"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("Enter selects / Esc cancels / q quits"),
        "{rendered}"
    );
    let buffer = render_app_to_buffer(&app, 100, 45);
    let (_, message_y) =
        find_text_position(&buffer, "Quit winproc-tui?").expect("quit message should render");
    let (shortcut_x, shortcut_y) = find_text_position(&buffer, "Enter/q Quit  Esc Cancel")
        .expect("quit shortcuts should render");
    assert_eq!(shortcut_y, message_y + 2);
    assert_blank_row_above_text(&buffer, "Enter/q Quit  Esc Cancel");
    assert_eq!(buffer[(shortcut_x, shortcut_y)].fg, app.theme().warning);
    assert!(
        buffer[(shortcut_x, shortcut_y)]
            .modifier
            .contains(Modifier::BOLD)
    );
    let esc_x = shortcut_x + "Enter/q Quit  ".chars().count() as u16;
    assert_eq!(buffer[(esc_x, shortcut_y)].fg, app.theme().warning);
    assert!(
        buffer[(esc_x, shortcut_y)]
            .modifier
            .contains(Modifier::BOLD)
    );
}

#[test]
fn quit_confirmation_dialog_keeps_shortcuts_on_one_row_on_narrow_screens() {
    let mut app = make_test_app(1, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();

    let rendered = render_app_to_text(&app, 40, 24);
    assert!(rendered.contains("Quit winproc-tui?"), "{rendered}");
    assert!(
        !rendered.contains("Close winproc-tui and return to terminal."),
        "{rendered}"
    );
    assert!(!rendered.contains("[ Quit ]"), "{rendered}");
    assert!(!rendered.contains("[ Cancel ]"), "{rendered}");
    assert!(rendered.contains("Enter/q Quit  Esc Cancel"), "{rendered}");
    assert!(
        !rendered.contains("Enter selects / Esc cancels / q quits"),
        "{rendered}"
    );
}

#[test]
fn quit_confirmation_dialog_mentions_recording_flush_when_active() {
    let mut app = make_test_app(1, 10);
    track_process_name(&mut app, "proc-0");
    let path = unique_recording_path("quit");
    app.recording_path_draft = path.display().to_string();
    app.recording_path_cursor = app.recording_path_draft.len();
    app.show_recording_path_dialog = true;

    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();

    let rendered = render_app_to_text(&app, 100, 45);
    assert!(rendered.contains("Stop recording and quit?"), "{rendered}");
    assert!(
        rendered.contains("The log will be flushed before exit."),
        "{rendered}"
    );
    assert!(
        !rendered.contains("Recording is active. The log will be flushed first."),
        "{rendered}"
    );

    app.stop_recording().unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn quit_confirmation_enter_confirms_quit() {
    let mut app = make_test_app(1, 10);

    app.request_quit_confirmation();
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert!(!app.show_quit_confirmation);
    assert!(app.should_quit);
}

#[test]
fn quit_confirmation_ignores_navigation_keys() {
    let mut app = make_test_app(1, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();
    for code in [KeyCode::Tab, KeyCode::Right, KeyCode::Left] {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap();
        assert!(app.show_quit_confirmation);
        assert!(!app.should_quit);
    }
}

#[test]
fn quit_confirmation_q_confirms_and_esc_cancels() {
    let mut app = make_test_app(1, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.show_quit_confirmation);
    assert!(!app.should_quit);

    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();

    assert!(!app.show_quit_confirmation);
    assert!(app.should_quit);
}

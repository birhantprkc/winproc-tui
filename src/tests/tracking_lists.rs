use super::support::{
    find_text_position, left_click, make_test_app, render_app_to_buffer, render_app_to_text,
};
use crate::app;
use crate::config;
use crate::ui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

#[test]
fn tracked_lists_uses_keyboard_actions_without_buttons() {
    let mut app = make_test_app(2, 10);
    app.open_tracked_lists();
    let rendered = render_app_to_text(&app, 120, 45);

    assert!(!rendered.contains("[ Save ]"), "{rendered}");
    assert!(!rendered.contains("[ Close ]"), "{rendered}");
    assert!(rendered.contains("↑/↓ Select"), "{rendered}");
    assert!(!rendered.contains("Up/Down Select"), "{rendered}");
    assert!(rendered.contains("Enter Load"), "{rendered}");
    assert!(rendered.contains("Esc Close"), "{rendered}");
}

#[test]
fn tracked_lists_separates_loading_and_saving() {
    let mut app = make_test_app(2, 10);
    app.watch_list = vec!["chrome.exe".to_string()];
    app.runtime.active_tracked_list = Some("Browser".to_string());
    app.open_tracked_lists();

    let rendered = render_app_to_text(&app, 120, 45);

    assert!(rendered.contains("LOAD TRACKING LIST"), "{rendered}");
    assert!(
        rendered.contains("Select a Tracking List to load."),
        "{rendered}"
    );
    assert!(rendered.contains("Empty (default)"), "{rendered}");
    assert!(
        rendered.contains("SAVE CURRENT TRACKING LIST"),
        "{rendered}"
    );
    assert!(rendered.contains("Current: Browser"), "{rendered}");
    assert!(rendered.contains("List name:  Browser"), "{rendered}");
    assert!(rendered.contains("TRACKING LIST STARTUP"), "{rendered}");
    assert!(rendered.contains("(*) Resume last"), "{rendered}");
    assert!(rendered.contains("( ) Choose list"), "{rendered}");
    assert!(rendered.contains("( ) Start empty"), "{rendered}");
    assert!(!rendered.contains("[ Rename ]"), "{rendered}");
    assert!(!rendered.contains("[ Delete ]"), "{rendered}");
}

#[test]
fn tracked_lists_dims_selected_row_when_list_loses_focus() {
    let mut app = make_test_app(1, 10);
    app.open_tracked_lists();

    let focused = render_app_to_buffer(&app, 120, 45);
    let (x, y) = find_text_position(&focused, "Empty (default)")
        .expect("selected Tracking List row should render");
    assert_eq!(focused[(x, y)].bg, app.theme().highlight);

    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    let unfocused = render_app_to_buffer(&app, 120, 45);
    assert_eq!(unfocused[(x, y)].bg, app.theme().selection);
    assert_ne!(focused[(x, y)].bg, unfocused[(x, y)].bg);
}

#[test]
fn tracked_lists_rows_preview_process_names_instead_of_counts() {
    let mut app = make_test_app(1, 10);
    app.runtime.active_tracked_list = Some("Browser".to_string());
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "Browser".to_string(),
        processes: vec!["chrome.exe".to_string(), "node.exe".to_string()],
    }];
    app.open_tracked_lists();

    let rendered = render_app_to_text(&app, 120, 45);
    let row = rendered
        .lines()
        .find(|line| line.contains("Browser (*)"))
        .expect("saved Tracking List row should render");

    assert!(row.contains("chrome.exe, node.exe"), "{row}");
    assert!(!row.contains("2 processes"), "{row}");
}

#[test]
fn tracked_lists_enter_loads_selected_saved_list() {
    let mut app = make_test_app(1, 10);
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "API".to_string(),
        processes: vec!["api.exe".to_string()],
    }];
    app.open_tracked_lists();
    app.move_tracked_list_selection_down(1);

    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.watch_list, vec!["api.exe"]);
    assert!(app.tracked_lists_dialog.is_none());
}

#[test]
fn tracked_lists_builtin_empty_is_virtual_and_active_only_for_empty_working_list() {
    let mut app = make_test_app(1, 10);
    app.open_tracked_lists();

    let rendered = render_app_to_text(&app, 120, 45);
    let empty_row = rendered
        .lines()
        .find(|line| line.contains("Empty (default)"))
        .expect("built-in empty row should render");
    assert!(empty_row.contains("Empty (default) (*)"), "{empty_row}");
    assert!(rendered.contains("Enter Load"), "{rendered}");
    assert!(!rendered.contains("New Empty"), "{rendered}");
    assert!(app.runtime.saved_tracked_lists.is_empty());

    app.watch_list = vec!["worker.exe".to_string()];
    app.normalized_watch_names = ["worker.exe".to_string()].into_iter().collect();
    let rendered = render_app_to_text(&app, 120, 45);
    let empty_row = rendered
        .lines()
        .find(|line| line.contains("Empty (default)"))
        .expect("built-in empty row should render");
    assert!(!empty_row.contains("(*)"), "{empty_row}");

    app.watch_list.clear();
    app.normalized_watch_names.clear();
    app.runtime.active_tracked_list = Some("Saved empty".to_string());
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "Saved empty".to_string(),
        processes: Vec::new(),
    }];
    let rendered = render_app_to_text(&app, 120, 45);
    let empty_row = rendered
        .lines()
        .find(|line| line.contains("Empty (default)"))
        .expect("built-in empty row should render");
    assert!(!empty_row.contains("(*)"), "{empty_row}");
    assert!(rendered.contains("Saved empty (*)"), "{rendered}");
}

#[test]
fn tracked_lists_builtin_empty_loads_with_enter_and_preserves_tracked_only() {
    let mut app = make_test_app(1, 10);
    app.watch_list = vec!["api.exe".to_string()];
    app.normalized_watch_names = ["api.exe".to_string()].into_iter().collect();
    app.watch_enabled = true;
    app.runtime.active_tracked_list = Some("API".to_string());
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "API".to_string(),
        processes: vec!["api.exe".to_string()],
    }];
    app.open_tracked_lists();
    app.move_tracked_list_selection_home();

    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert!(app.watch_list.is_empty());
    assert!(app.watch_enabled);
    assert_eq!(app.runtime.active_tracked_list, None);
    assert_eq!(app.runtime.saved_tracked_lists.len(), 1);
    assert_eq!(app.runtime.saved_tracked_lists[0].name, "API");
    assert!(app.tracked_lists_dialog.is_none());
}

#[test]
fn tracked_lists_builtin_empty_loads_with_mouse() {
    let screen = Rect::new(0, 0, 120, 45);
    let mut app = make_test_app(1, 10);
    app.watch_list = vec!["api.exe".to_string()];
    app.normalized_watch_names = ["api.exe".to_string()].into_iter().collect();
    app.open_tracked_lists();
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let (x, y) =
        find_text_position(&buffer, "Empty (default)").expect("built-in empty row should render");

    app.on_mouse(left_click(x + 1, y), screen);

    assert!(app.watch_list.is_empty());
    assert!(app.tracked_lists_dialog.is_none());
}

#[test]
fn tracked_lists_builtin_empty_cannot_be_renamed_deleted_or_overwritten() {
    let mut app = make_test_app(1, 10);
    app.open_tracked_lists();

    app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))
        .unwrap();
    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::Browse)
    ));
    app.on_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))
        .unwrap();
    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::Browse)
    ));

    app.focus_tracked_lists_save_name();
    for ch in "Empty (default)".chars() {
        app.push_tracked_list_save_name_char(ch);
    }
    app.save_current_tracked_list();

    let (_, _, error) = app
        .tracked_lists_save_name()
        .expect("save-name input should remain available");
    assert_eq!(
        error,
        Some("Empty (default) is built in and cannot be overwritten.")
    );
    assert!(app.runtime.saved_tracked_lists.is_empty());
}

#[test]
fn tracked_lists_named_list_cannot_be_renamed_to_builtin_empty_name() {
    let mut app = make_test_app(1, 10);
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "API".to_string(),
        processes: vec!["api.exe".to_string()],
    }];
    app.open_tracked_lists();
    app.move_tracked_list_selection_down(1);
    app.begin_tracked_list_rename();
    for _ in 0.."API".len() {
        app.pop_tracked_list_name_char();
    }
    for ch in "Empty (default)".chars() {
        app.push_tracked_list_name_char(ch);
    }

    app.commit_tracked_list_name_input();

    assert_eq!(app.runtime.saved_tracked_lists[0].name, "API");
    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::NameInput { error: Some(error), .. })
            if error.contains("cannot be overwritten")
    ));
}

#[test]
fn tracked_lists_plain_n_no_longer_starts_empty() {
    let mut app = make_test_app(1, 10);
    app.watch_list = vec!["api.exe".to_string()];
    app.normalized_watch_names = ["api.exe".to_string()].into_iter().collect();
    app.open_tracked_lists();

    app.on_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.watch_list, vec!["api.exe"]);
    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::Browse)
    ));
}

#[test]
fn tracked_lists_tab_cycles_list_name_and_startup_controls() {
    let mut app = make_test_app(1, 10);
    app.open_tracked_lists();
    let expected = [(true, false), (false, true), (false, false)];

    for (save_name_focused, startup_focused) in expected {
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
            .unwrap();
        assert_eq!(app.tracked_lists_save_name_focused(), save_name_focused);
        assert_eq!(app.tracked_lists_startup_focused(), startup_focused);
    }

    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert!(app.tracked_lists_startup_focused());
}

#[test]
fn tracked_lists_f2_opens_rename_and_plain_r_d_do_nothing() {
    let mut app = make_test_app(1, 10);
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "API".to_string(),
        processes: vec!["api.exe".to_string()],
    }];
    app.open_tracked_lists();
    app.move_tracked_list_selection_down(1);

    app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))
        .unwrap();
    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::NameInput { .. })
    ));

    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE))
        .unwrap();
    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::Browse)
    ));
}

#[test]
fn tracked_lists_delete_key_opens_delete_confirmation() {
    let mut app = make_test_app(1, 10);
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "API".to_string(),
        processes: vec!["api.exe".to_string()],
    }];
    app.open_tracked_lists();
    app.move_tracked_list_selection_down(1);

    app.on_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))
        .unwrap();

    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::ConfirmDelete { name, .. }) if name == "API"
    ));
    let rendered = render_app_to_text(&app, 120, 45);
    assert!(
        rendered.contains("Enter/Esc/n Cancel  y Delete"),
        "{rendered}"
    );
}

#[test]
fn tracked_lists_save_name_accepts_keyboard_input_and_enter() {
    let mut app = make_test_app(1, 10);
    app.watch_list = vec!["api.exe".to_string()];
    app.open_tracked_lists();
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert!(app.tracked_lists_save_name_focused());

    for ch in "API".chars() {
        app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            .unwrap();
    }
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.runtime.active_tracked_list.as_deref(), Some("API"));
    assert_eq!(app.runtime.saved_tracked_lists.len(), 1);
    assert_eq!(
        app.runtime.saved_tracked_lists[0].processes,
        vec!["api.exe"]
    );
}

#[test]
fn tracked_lists_save_name_focuses_with_mouse() {
    let screen = Rect::new(0, 0, 120, 45);
    let mut app = make_test_app(1, 10);
    app.runtime.active_tracked_list = Some("API".to_string());
    app.open_tracked_lists();
    let input = ui::tracked_list_save_name_area_for_screen(screen)
        .expect("save-name input should have an area");

    app.on_mouse(left_click(input.x + 1, input.y), screen);

    assert!(app.tracked_lists_save_name_focused());
}

#[test]
fn tracked_list_startup_changes_with_keyboard_and_mouse() {
    let screen = Rect::new(0, 0, 120, 45);
    let mut app = make_test_app(1, 10);
    app.open_tracked_lists();
    for _ in 0..2 {
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
            .unwrap();
    }
    assert!(app.tracked_lists_startup_focused());

    app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(
        app.runtime.tracked_list_startup,
        config::TrackedListStartup::ChooseList
    );

    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let (x, y) = find_text_position(&buffer, "( ) Start empty")
        .expect("Start empty radio option should render");
    app.on_mouse(left_click(x + 2, y), screen);
    assert_eq!(
        app.runtime.tracked_list_startup,
        config::TrackedListStartup::StartEmpty
    );
    assert!(app.tracked_lists_startup_focused());

    let rendered = render_app_to_text(&app, screen.width, screen.height);
    assert!(rendered.contains("Enter/Esc Close"), "{rendered}");
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(app.tracked_lists_dialog.is_none());
}

#[test]
fn tracked_list_delete_confirmation_requires_keyboard_confirmation() {
    let screen = Rect::new(0, 0, 120, 45);
    let mut app = make_test_app(1, 10);
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "API".to_string(),
        processes: vec!["api.exe".to_string()],
    }];
    app.open_tracked_lists();
    app.move_tracked_list_selection_down(1);
    app.request_delete_selected_tracked_list();
    app.on_mouse(left_click(screen.width / 2, screen.height / 2), screen);

    assert_eq!(app.runtime.saved_tracked_lists.len(), 1);
    app.on_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE))
        .unwrap();

    assert!(app.runtime.saved_tracked_lists.is_empty());
    assert!(matches!(
        app.tracked_lists_view(),
        Some(app::TrackedListsView::Browse)
    ));
}

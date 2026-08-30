use std::{path::PathBuf, sync::mpsc::TryRecvError};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::Modifier};

use super::support::{
    find_text_position, left_click, make_test_app, make_test_app_with_worker, mouse_move,
    render_app_to_buffer, render_app_to_text, track_process_name, unique_recording_path,
};
use crate::{
    app::{App, AppActivity, ProcessViewMode, profiles::InvestigationProfilesView},
    samplers::{SampleRequest, SamplingWorker},
    ui::{self, main_menu_area, main_menu_item_area},
};

fn press(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap();
}

fn menu_labels(app: &App) -> Vec<String> {
    app.main_menu_rows()
        .iter()
        .map(|row| app.main_menu_row_label(*row))
        .collect()
}

fn start_recording(app: &mut App, label: &str) -> PathBuf {
    let path = unique_recording_path(label);
    let _ = std::fs::remove_file(&path);
    track_process_name(app, "proc-0");
    app.recording_path_draft = path.display().to_string();
    app.recording_path_cursor = app.recording_path_draft.len();
    app.show_recording_path_dialog = true;
    app.confirm_recording_path().unwrap();
    assert_eq!(app.activity(), AppActivity::Recording);
    path
}

#[test]
fn menu_uses_the_exact_activity_specific_item_sets() {
    let mut app = make_test_app(1, 10);
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        menu_labels(&app),
        vec![
            "Profile ▸",
            "Columns",
            "View ▸",
            "Start Recording",
            "Log ▸",
            "Config ▸",
            "Help",
            "Quit"
        ]
    );
    assert_eq!(app.main_menu_selected, 0);
    assert!(
        !menu_labels(&app)
            .iter()
            .any(|label| label.contains("Settings"))
    );
    assert!(menu_labels(&app).iter().all(|label| !label.contains("...")));

    app.dismiss_main_menu();
    let path = start_recording(&mut app, "main-menu-items");
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        menu_labels(&app),
        vec![
            "Profile ▸",
            "Columns",
            "View ▸",
            "Stop Recording",
            "Config ▸",
            "Help",
            "Quit"
        ]
    );
    assert_eq!(app.main_menu_selected, 0);
    press(&mut app, KeyCode::Right);
    assert_eq!(
        menu_labels(&app),
        vec![
            "Profile ▾",
            "Open",
            "Columns",
            "View ▸",
            "Stop Recording",
            "Config ▸",
            "Help",
            "Quit"
        ]
    );
    assert!(!menu_labels(&app).iter().any(|label| label == "Save"));
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Right);
    assert!(
        menu_labels(&app)
            .iter()
            .any(|label| label == "[ ] Tree view")
    );
    app.dismiss_main_menu();
    app.stop_recording().unwrap();
    let _ = std::fs::remove_file(path);

    app.log_view_path = Some(PathBuf::from("C:/logs/example.log"));
    press(&mut app, KeyCode::Esc);
    assert_eq!(
        menu_labels(&app),
        vec![
            "Profile ▸",
            "Columns",
            "View ▸",
            "Log ▸",
            "Config ▸",
            "Help",
            "Quit"
        ]
    );
    assert_eq!(app.main_menu_selected, 0);
    press(&mut app, KeyCode::Right);
    assert_eq!(menu_labels(&app)[1], "Open");
    assert!(!menu_labels(&app).iter().any(|label| label == "Save"));
}

#[test]
fn menu_expands_profile_and_toggles_view_checkboxes_inline() {
    let mut app = make_test_app(1, 10);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Right);

    assert_eq!(app.main_menu_selected, 1);
    assert_eq!(
        menu_labels(&app),
        vec![
            "Profile ▾",
            "Open",
            "Save",
            "Save As",
            "Columns",
            "View ▸",
            "Start Recording",
            "Log ▸",
            "Config ▸",
            "Help",
            "Quit"
        ]
    );

    press(&mut app, KeyCode::Up);
    assert_eq!(app.main_menu_selected, 0);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.main_menu_selected, 1);
    assert_eq!(menu_labels(&app)[0], "Profile ▾");

    press(&mut app, KeyCode::Left);
    assert_eq!(app.main_menu_selected, 0);
    assert_eq!(menu_labels(&app)[0], "Profile ▸");

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(!app.is_main_menu_open());
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::NameInput { .. })
    ));
    app.close_investigation_profiles();

    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.main_menu_selected, 3);
    assert_eq!(menu_labels(&app)[3], "[ ] Tracked-only");
    assert_eq!(menu_labels(&app)[4], "[ ] Tree view");
    press(&mut app, KeyCode::Char(' '));
    assert!(app.watch_enabled);
    assert!(app.is_main_menu_open());
    assert_eq!(menu_labels(&app)[3], "[x] Tracked-only");
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.process_view_mode, ProcessViewMode::Tree);
    assert!(app.is_main_menu_open());
    press(&mut app, KeyCode::Esc);

    app.log_view_path = Some(PathBuf::from("C:/logs/example.log"));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Right);
    assert_eq!(menu_labels(&app)[3], "[x] Tracked-only");
    assert!(!menu_labels(&app).iter().any(|label| label == "Tree view"));
    assert!(
        !menu_labels(&app)
            .iter()
            .any(|label| label.contains("Tree view"))
    );
}

#[test]
fn menu_keeps_multiple_parents_expanded_and_left_collapses_the_selected_parent() {
    let mut app = make_test_app(1, 10);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Right);

    app.main_menu_selected = menu_labels(&app)
        .iter()
        .position(|label| label == "View ▸")
        .expect("View parent");
    press(&mut app, KeyCode::Right);
    assert!(menu_labels(&app).iter().any(|label| label == "Profile ▾"));
    assert!(menu_labels(&app).iter().any(|label| label == "View ▾"));

    app.main_menu_selected = menu_labels(&app)
        .iter()
        .position(|label| label == "Log ▸")
        .expect("Log parent");
    press(&mut app, KeyCode::Right);
    assert!(menu_labels(&app).iter().any(|label| label == "Profile ▾"));
    assert!(menu_labels(&app).iter().any(|label| label == "View ▾"));
    assert!(menu_labels(&app).iter().any(|label| label == "Log ▾"));

    app.main_menu_selected = menu_labels(&app)
        .iter()
        .position(|label| label == "View ▾")
        .expect("expanded View parent");
    press(&mut app, KeyCode::Left);
    assert_eq!(menu_labels(&app)[app.main_menu_selected], "View ▸");
    assert!(menu_labels(&app).iter().any(|label| label == "Profile ▾"));
    assert!(menu_labels(&app).iter().any(|label| label == "Log ▾"));
}

#[test]
fn menu_routes_profiles_and_columns_to_existing_dialogs() {
    let mut app = make_test_app(1, 10);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(app.investigation_profiles_view().is_some());
    app.close_investigation_profiles();

    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(app.show_column_picker);
    press(&mut app, KeyCode::Esc);
    assert!(!app.show_column_picker);
    assert!(!app.is_main_menu_open());
}

#[test]
fn menu_config_opens_startup_behavior() {
    let mut app = make_test_app(1, 10);
    app.open_main_menu();
    app.main_menu_selected = menu_labels(&app)
        .iter()
        .position(|label| label == "Config ▸")
        .expect("Config parent");

    press(&mut app, KeyCode::Enter);
    assert_eq!(
        menu_labels(&app)[app.main_menu_selected],
        "Startup Behavior"
    );
    press(&mut app, KeyCode::Enter);

    assert!(!app.is_main_menu_open());
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::Startup { .. })
    ));
    let rendered = render_app_to_text(&app, 100, 45);
    assert!(rendered.contains("STARTUP BEHAVIOR"), "{rendered}");
    press(&mut app, KeyCode::Esc);
    assert!(app.investigation_profiles_dialog.is_none());
}

#[test]
fn escape_opens_and_closes_menu_without_quitting_or_leaving_log_view() {
    let mut app = make_test_app(1, 10);
    app.log_view_path = Some(PathBuf::from("C:/logs/example.log"));

    press(&mut app, KeyCode::Esc);
    assert!(app.is_main_menu_open());
    assert_eq!(app.activity(), AppActivity::LogView);
    assert!(!app.show_quit_confirmation);

    press(&mut app, KeyCode::Esc);
    assert!(!app.is_main_menu_open());
    assert_eq!(app.activity(), AppActivity::LogView);
    assert!(!app.show_quit_confirmation);
}

#[test]
fn menu_navigation_and_actions_reuse_existing_flows() {
    let mut app = make_test_app(1, 10);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::End);
    assert_eq!(app.main_menu_selected, 7);
    press(&mut app, KeyCode::Home);
    assert_eq!(app.main_menu_selected, 0);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.main_menu_selected, 7);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.main_menu_selected, 0);
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.main_menu_selected, 0);
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.main_menu_selected, 6);
    press(&mut app, KeyCode::Enter);
    assert!(!app.is_main_menu_open());
    assert!(app.show_help);

    press(&mut app, KeyCode::Esc);
    assert!(!app.show_help);
    assert!(!app.is_main_menu_open());

    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('q'));
    assert!(!app.is_main_menu_open());
    assert!(!app.show_quit_confirmation);
    assert!(app.should_quit);
}

#[test]
fn menu_quit_is_immediate_in_live_and_log_view_but_confirms_recording() {
    for log_view in [false, true] {
        let mut app = make_test_app(1, 10);
        if log_view {
            app.log_view_path = Some(PathBuf::from("C:/logs/example.log"));
        }
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::End);
        press(&mut app, KeyCode::Enter);

        assert!(app.should_quit);
        assert!(!app.show_quit_confirmation);
        assert!(!app.is_main_menu_open());
    }

    let mut direct_q = make_test_app(1, 10);
    press(&mut direct_q, KeyCode::Char('q'));
    assert!(!direct_q.should_quit);
    assert!(direct_q.show_quit_confirmation);

    let mut recording = make_test_app(1, 10);
    let path = start_recording(&mut recording, "main-menu-quit-confirmation");
    press(&mut recording, KeyCode::Esc);
    press(&mut recording, KeyCode::End);
    press(&mut recording, KeyCode::Enter);

    assert!(!recording.should_quit);
    assert!(recording.show_quit_confirmation);
    assert!(!recording.is_main_menu_open());
    let rendered = render_app_to_text(&recording, 100, 45);
    assert!(rendered.contains("Stop recording and quit?"), "{rendered}");

    press(&mut recording, KeyCode::Esc);
    press(&mut recording, KeyCode::Esc);
    press(&mut recording, KeyCode::Right);
    press(&mut recording, KeyCode::Char('q'));
    assert!(recording.show_quit_confirmation);
    assert!(!recording.should_quit);
    press(&mut recording, KeyCode::Esc);
    recording.stop_recording().unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn menu_routes_activity_transitions_and_confirmations() {
    let mut app = make_test_app(1, 10);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(app.show_recording_no_tracked_warning);
    assert!(!app.is_main_menu_open());
    press(&mut app, KeyCode::Esc);

    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Enter);
    assert!(app.show_log_list);
    assert!(!app.is_main_menu_open());
    app.close_log_list();

    app.log_view_path = Some(PathBuf::from("C:/logs/example.log"));
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.activity(), AppActivity::Live);
    assert!(!app.is_main_menu_open());

    let path = start_recording(&mut app, "main-menu-stop");
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(app.show_recording_stop_confirmation);
    assert_eq!(app.activity(), AppActivity::Recording);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.activity(), AppActivity::Recording);
    app.stop_recording().unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn existing_modal_and_editing_escape_handlers_do_not_open_menu() {
    let mut app = make_test_app(1, 10);
    app.show_help = true;
    press(&mut app, KeyCode::Esc);
    assert!(!app.show_help);
    assert!(!app.is_main_menu_open());

    app.request_quit_confirmation();
    press(&mut app, KeyCode::Esc);
    assert!(!app.show_quit_confirmation);
    assert!(!app.is_main_menu_open());

    app.begin_filter_edit();
    press(&mut app, KeyCode::Esc);
    assert!(!app.is_filter_editing());
    assert!(!app.is_main_menu_open());

    app.begin_process_jump_edit();
    press(&mut app, KeyCode::Esc);
    assert!(!app.is_process_jump_editing());
    assert!(!app.is_main_menu_open());
}

#[test]
fn menu_is_compact_without_title_or_footer_guidance_and_keeps_hover_style() {
    let screen = Rect::new(0, 0, 80, 24);
    let mut app = make_test_app(1, 10);
    press(&mut app, KeyCode::Esc);

    let rendered = render_app_to_text(&app, screen.width, screen.height);
    assert!(rendered.contains("[MENU]"), "{rendered}");
    assert!(!rendered.contains("↑/↓ Select"), "{rendered}");
    assert!(!rendered.contains("Settings"), "{rendered}");
    let popup = main_menu_area(screen, &app);
    assert_eq!(popup.x, screen.x);
    assert_eq!(popup.y, screen.y.saturating_add(1));
    assert_eq!(popup.height, app.main_menu_rows().len() as u16 + 2);
    let collapsed_width = popup.width;
    let selected = main_menu_item_area(screen, &app, 0).expect("selected row");
    let title_row = Rect::new(popup.x, popup.y, popup.width, 1);
    for (theme_index, theme) in ui::THEMES.iter().copied().enumerate() {
        app.theme_index = theme_index;
        let selected_buffer = render_app_to_buffer(&app, screen.width, screen.height);
        for x in selected.x..selected.right() {
            assert_eq!(
                selected_buffer[(x, selected.y)].bg,
                theme.table_selection_surface,
                "theme={theme_index}, x={x}"
            );
            assert_ne!(
                selected_buffer[(x, selected.y)].bg,
                theme.highlight,
                "theme={theme_index}, x={x}"
            );
        }
        for x in title_row.x.saturating_add(1)..title_row.right().saturating_sub(1) {
            assert_eq!(
                selected_buffer[(x, title_row.y)].symbol(),
                "━",
                "theme={theme_index}, x={x}"
            );
        }
    }

    app.theme_index = 0;
    press(&mut app, KeyCode::Right);
    assert_eq!(main_menu_area(screen, &app).width, collapsed_width);
    press(&mut app, KeyCode::Left);
    let large_screen = Rect::new(0, 0, 100, 45);
    let large_popup = main_menu_area(large_screen, &app);
    let large_buffer = render_app_to_buffer(&app, large_screen.width, large_screen.height);
    assert_eq!(
        large_buffer[(large_popup.x, large_popup.y)].fg,
        app.theme().focus_border
    );

    app.on_mouse(mouse_move(selected.x, selected.y), screen);
    let selected_hovered = render_app_to_buffer(&app, screen.width, screen.height);
    assert_eq!(
        selected_hovered[(selected.x, selected.y)].bg,
        app.theme().table_selection_surface
    );

    let help_index = menu_labels(&app)
        .iter()
        .position(|label| label == "Help")
        .expect("Help index");
    let help = main_menu_item_area(screen, &app, help_index).expect("Help row");
    app.on_mouse(mouse_move(help.x, help.y), screen);
    assert_eq!(app.main_menu_hovered, Some(help_index));
    let hovered = render_app_to_buffer(&app, screen.width, screen.height);
    assert_eq!(hovered[(help.x, help.y)].bg, app.theme().focus_surface);
    assert!(hovered[(help.x, help.y)].modifier.contains(Modifier::BOLD));

    app.on_mouse(left_click(help.x, help.y), screen);
    assert!(!app.is_main_menu_open());
    assert!(app.show_help);
}

#[test]
fn menu_parents_expand_and_children_activate_with_the_mouse() {
    let screen = Rect::new(0, 0, 80, 24);
    let mut app = make_test_app(1, 10);
    app.open_main_menu();

    let profile = main_menu_item_area(screen, &app, 0).expect("Profile row");
    app.on_mouse(left_click(profile.x, profile.y), screen);
    assert_eq!(menu_labels(&app)[0], "Profile ▾");
    assert_eq!(app.main_menu_selected, 1);

    let open = main_menu_item_area(screen, &app, 1).expect("Open child row");
    app.on_mouse(left_click(open.x, open.y), screen);
    assert!(!app.is_main_menu_open());
    assert!(app.investigation_profiles_view().is_some());
    app.close_investigation_profiles();

    app.open_main_menu();
    let profile = main_menu_item_area(screen, &app, 0).expect("Profile row");
    app.on_mouse(left_click(profile.x, profile.y), screen);
    let profile = main_menu_item_area(screen, &app, 0).expect("expanded Profile row");
    app.on_mouse(left_click(profile.x, profile.y), screen);
    assert_eq!(menu_labels(&app)[0], "Profile ▸");
    assert_eq!(app.main_menu_selected, 0);

    let view_index = menu_labels(&app)
        .iter()
        .position(|label| label == "View ▸")
        .expect("View parent index");
    let view = main_menu_item_area(screen, &app, view_index).expect("View row");
    app.on_mouse(left_click(view.x, view.y), screen);
    let tracked_only_index = menu_labels(&app)
        .iter()
        .position(|label| label == "[ ] Tracked-only")
        .expect("Tracked-only child index");
    let tracked_only =
        main_menu_item_area(screen, &app, tracked_only_index).expect("Tracked-only row");
    app.on_mouse(left_click(tracked_only.x, tracked_only.y), screen);
    assert!(app.watch_enabled);
    assert!(app.is_main_menu_open());
    assert_eq!(menu_labels(&app)[tracked_only_index], "[x] Tracked-only");
}

#[test]
fn header_menu_button_uses_shared_geometry_hover_and_opening() {
    let screen = Rect::new(0, 0, 80, 24);
    let mut app = make_test_app(1, 10);
    let button = ui::header_menu_area_for_screen(screen, &app).expect("header MENU button");

    let rendered = render_app_to_text(&app, screen.width, screen.height);
    assert!(rendered.contains("LIVE"), "{rendered}");
    assert!(rendered.contains("[MENU]"), "{rendered}");
    assert_eq!(button.x, screen.x);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let (menu_x, menu_y) = find_text_position(&buffer, "[MENU]").expect("menu label position");
    let (live_x, live_y) = find_text_position(&buffer, "LIVE").expect("activity position");
    assert_eq!((menu_x, menu_y), (screen.x, screen.y));
    assert_eq!(live_y, screen.y);
    assert!(live_x >= button.right());

    app.on_mouse(mouse_move(button.x, button.y), screen);
    assert!(app.header_menu_hovered);
    let hovered = render_app_to_buffer(&app, screen.width, screen.height);
    assert_eq!(hovered[(button.x, button.y)].bg, app.theme().focus_surface);
    assert!(
        hovered[(button.x, button.y)]
            .modifier
            .contains(Modifier::BOLD)
    );

    app.on_mouse(left_click(button.x, button.y), screen);
    assert!(app.is_main_menu_open());
    assert_eq!(app.main_menu_selected, 0);
    let active = render_app_to_buffer(&app, screen.width, screen.height);
    assert_eq!(
        active[(button.x, button.y)].bg,
        app.theme().table_selection_surface
    );
}

#[test]
fn menu_hit_testing_uses_only_visible_item_rows() {
    let screen = Rect::new(0, 0, 80, 24);
    let mut app = make_test_app(1, 10);
    app.open_main_menu();
    let popup = main_menu_area(screen, &app);
    let selected_before = app.main_menu_selected;

    assert!(ui::main_menu_index_at(screen, &app, popup.x, popup.y).is_none());
    app.on_mouse(left_click(popup.x, popup.y), screen);
    assert!(app.is_main_menu_open());
    assert_eq!(app.main_menu_selected, selected_before);

    app.activate_main_menu_at(usize::MAX).unwrap();
    assert!(app.is_main_menu_open());
    assert!(!app.should_quit);
}

#[test]
fn expanded_menu_remains_usable_on_a_narrow_screen() {
    let screen = Rect::new(0, 0, 24, 20);
    let mut app = make_test_app(1, 10);
    app.open_main_menu();
    press(&mut app, KeyCode::Right);
    app.main_menu_selected = menu_labels(&app)
        .iter()
        .position(|label| label == "View ▸")
        .expect("View parent");
    press(&mut app, KeyCode::Right);
    app.main_menu_selected = menu_labels(&app)
        .iter()
        .position(|label| label == "Log ▸")
        .expect("Log parent");
    press(&mut app, KeyCode::Right);

    let rendered = render_app_to_text(&app, screen.width, screen.height);
    assert!(rendered.contains("MENU"), "{rendered}");
    assert!(rendered.contains("Profile"), "{rendered}");
    assert!(rendered.contains("Open"), "{rendered}");
    assert!(rendered.contains("View"), "{rendered}");
    assert!(rendered.contains("Tracked-only"), "{rendered}");
    assert!(rendered.contains("Log"), "{rendered}");
    assert!(rendered.contains("Open"), "{rendered}");
    assert!(app.main_menu_selected < app.main_menu_rows().len());
}

#[test]
fn menu_does_not_block_sampling_or_recording_frame_writes() {
    let (sampling_worker, request_rx, _result_tx) = SamplingWorker::test_pair();
    let mut app = make_test_app_with_worker(1, 10, sampling_worker);
    app.open_main_menu();

    assert!(!app.request_sample().unwrap());
    assert_eq!(request_rx.try_recv(), Ok(SampleRequest::Sample));
    assert!(app.is_main_menu_open());

    app.sampling_in_progress = false;
    app.dismiss_main_menu();
    let path = start_recording(&mut app, "main-menu-recording-continues");
    app.open_main_menu();
    app.write_current_recording_frame().unwrap();
    assert_eq!(app.activity(), AppActivity::Recording);
    assert!(app.is_main_menu_open());
    app.dismiss_main_menu();
    app.stop_recording().unwrap();
    let _ = std::fs::remove_file(path);

    assert_eq!(request_rx.try_recv(), Err(TryRecvError::Empty));
}

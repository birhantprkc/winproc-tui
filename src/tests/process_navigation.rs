use super::support::{
    add_test_graph, assign_private_graph, find_text_position, left_click, make_test_app,
    render_app_to_buffer, render_app_to_text, track_process_name,
};
use crate::app;
use crate::app::{DetailsMetric, FocusedPanel, GraphSlot};
use crate::model;
use crate::model::{MetricColumn, SortColumn};
use crate::ui;
use crate::ui::{
    details_graph_area_for_app, main_panel_areas, main_panel_areas_for_app,
    process_kill_dialog_area,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Modifier;

#[test]
fn tab_cycles_focus_through_visible_panels() {
    let mut app = make_test_app(1, 10);

    assert_eq!(app.focused_panel, FocusedPanel::Processes);
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::System);
    assert_eq!(app.resource_panel, app::ResourcePanel::Memory);
    assert_eq!(app.status, "Focus: MEM");

    let identity = app.selected_visible_process_identity().unwrap();
    app.add_or_reveal_graph_source(
        GraphSlot::process(identity.clone(), DetailsMetric::Private),
        FocusedPanel::Processes,
    );
    app.add_or_reveal_graph_source(
        GraphSlot::process(identity, DetailsMetric::Workset),
        FocusedPanel::Processes,
    );
    let active_id = app.active_graph_id;
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::System);
    assert_eq!(app.resource_panel, app::ResourcePanel::Gpu);
    assert_eq!(app.status, "Focus: GPU");
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::SystemActivity);
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::Cpu);
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::Processes);
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::DetailsGraph);
    assert_eq!(app.active_graph_id, active_id);
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::DetailsSamples);
    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::System);
    assert_eq!(app.resource_panel, app::ResourcePanel::Memory);
    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::DetailsSamples);
    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::DetailsGraph);
    assert_eq!(app.active_graph_id, active_id);
    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::Processes);
    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::Cpu);
    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::SystemActivity);
    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::System);
    assert_eq!(app.resource_panel, app::ResourcePanel::Gpu);
    assert_eq!(app.status, "Focus: GPU");
    app.on_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.focused_panel, FocusedPanel::System);
    assert_eq!(app.resource_panel, app::ResourcePanel::Memory);
    assert_eq!(app.status, "Focus: MEM");
}

#[test]
fn process_page_size_uses_full_height_without_graphs_and_caps_with_graphs() {
    assert_eq!(
        main_panel_areas(Rect::new(0, 0, 120, 40), false, 30, false)
            .processes
            .page_size,
        27
    );
    assert_eq!(
        main_panel_areas(Rect::new(0, 0, 120, 60), true, 30, false)
            .processes
            .page_size,
        10
    );
}

#[test]
fn process_navigation_moves_up_after_overflowing_down() {
    let mut app = make_test_app(30, 10);
    app.move_selection_down(20);
    assert_eq!(app.process_table_state.selected(), Some(20));
    assert_eq!(app.process_table_state.offset(), 11);

    app.move_selection_up(1);
    assert_eq!(app.process_table_state.selected(), Some(19));
    assert_eq!(app.process_table_state.offset(), 11);
}

#[test]
fn process_navigation_page_moves_by_visible_rows() {
    let mut app = make_test_app(30, 10);
    app.move_selection_down(app.process_page_size);
    assert_eq!(app.process_table_state.selected(), Some(10));
    assert_eq!(app.process_table_state.offset(), 1);

    app.move_selection_up(app.process_page_size);
    assert_eq!(app.process_table_state.selected(), Some(0));
    assert_eq!(app.process_table_state.offset(), 0);
}

#[test]
fn process_navigation_home_and_end_jump_to_bounds() {
    let mut app = make_test_app(30, 10);
    app.select_last_row();
    assert_eq!(app.process_table_state.selected(), Some(29));
    assert_eq!(app.process_table_state.offset(), 20);

    app.select_first_row();
    assert_eq!(app.process_table_state.selected(), Some(0));
    assert_eq!(app.process_table_state.offset(), 0);
}

#[test]
fn process_shift_up_down_selects_live_row_range() {
    let mut app = make_test_app(5, 10);

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT))
        .unwrap();

    assert_eq!(app.process_table_state.selected(), Some(1));
    assert_eq!(app.selected_process_identities_count(), 2);
    assert!(
        app.selected_process_identities
            .contains(&model::ProcessIdentity::from_row(
                &app.snapshot.processes[0]
            ))
    );
    assert!(
        app.selected_process_identities
            .contains(&model::ProcessIdentity::from_row(
                &app.snapshot.processes[1]
            ))
    );

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.process_table_state.selected(), Some(2));
    assert_eq!(app.selected_process_identities_count(), 0);
}

#[test]
fn normal_process_navigation_does_not_keep_multi_selection_anchor() {
    let mut app = make_test_app(5, 10);

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.process_table_state.selected(), Some(1));
    assert!(app.process_selection_anchor.is_none());
    assert_eq!(app.selected_process_identities_count(), 0);

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT))
        .unwrap();

    assert_eq!(app.process_table_state.selected(), Some(2));
    assert_eq!(app.selected_process_identities_count(), 2);
    assert!(
        app.selected_process_identities
            .contains(&model::ProcessIdentity::from_row(
                &app.snapshot.processes[1]
            ))
    );
    assert!(
        app.selected_process_identities
            .contains(&model::ProcessIdentity::from_row(
                &app.snapshot.processes[2]
            ))
    );
}

#[test]
fn process_ctrl_space_toggles_discontiguous_live_rows() {
    let mut app = make_test_app(5, 10);

    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL))
        .unwrap();

    assert_eq!(app.process_table_state.selected(), Some(2));
    assert_eq!(app.selected_process_identities_count(), 2);
    assert!(
        app.selected_process_identities
            .contains(&model::ProcessIdentity::from_row(
                &app.snapshot.processes[0]
            ))
    );
    assert!(
        app.selected_process_identities
            .contains(&model::ProcessIdentity::from_row(
                &app.snapshot.processes[2]
            ))
    );

    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL))
        .unwrap();

    assert_eq!(app.selected_process_identities_count(), 1);
    assert!(
        !app.selected_process_identities
            .contains(&model::ProcessIdentity::from_row(
                &app.snapshot.processes[2]
            ))
    );
}

#[test]
fn process_kill_confirmation_keeps_every_selected_pid() {
    let mut app = make_test_app(3, 10);
    app.snapshot.processes[0].name = "same.exe".to_string();
    app.snapshot.processes[1].name = "same.exe".to_string();
    app.snapshot.processes[2].name = "other.exe".to_string();
    app.rebuild_visible_process_cache();
    app.clamp_process_table_state();

    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT))
        .unwrap();

    assert!(app.request_process_kill_confirmation());
    assert!(app.show_process_kill_confirmation);
    assert_eq!(app.process_kill_targets.len(), 3);
    assert_eq!(
        app.process_kill_targets
            .iter()
            .map(|target| target.pid)
            .collect::<Vec<_>>(),
        app.snapshot
            .processes
            .iter()
            .map(|process| process.pid)
            .collect::<Vec<_>>()
    );
}

#[test]
fn process_kill_confirmation_dialog_is_compact_and_keyboard_only() {
    let mut app = make_test_app(3, 10);
    app.focused_panel = FocusedPanel::Processes;
    app.snapshot.processes[0].name = "msedge.exe".to_string();
    app.rebuild_visible_process_cache();
    app.clamp_process_table_state();

    assert!(app.request_process_kill_confirmation());

    let screen = Rect::new(0, 0, 100, 45);
    let popup = process_kill_dialog_area(screen);
    assert_eq!(popup.width, 64);
    assert_eq!(popup.height, 9);

    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let shortcut = "Enter Kill  Esc Cancel";
    let (enter_x, shortcut_y) = find_text_position(&buffer, shortcut)
        .expect("process-kill shortcuts should follow footer formatting");
    assert_eq!(buffer[(popup.x, popup.y)].fg, app.theme().warning);
    assert_eq!(buffer[(enter_x, shortcut_y)].fg, app.theme().warning);
    assert!(
        buffer[(enter_x, shortcut_y)]
            .modifier
            .contains(Modifier::BOLD)
    );
    let esc_x = enter_x + "Enter Kill  ".chars().count() as u16;
    assert_eq!(buffer[(esc_x, shortcut_y)].fg, app.theme().warning);
    assert!(
        buffer[(esc_x, shortcut_y)]
            .modifier
            .contains(Modifier::BOLD)
    );
    let rendered = render_app_to_text(&app, screen.width, screen.height);
    assert!(!rendered.contains("[ Kill ]"), "{rendered}");
    assert!(!rendered.contains("[ Cancel ]"), "{rendered}");
    assert!(!rendered.contains("y Kill"), "{rendered}");
    assert!(!rendered.contains("n Cancel"), "{rendered}");
    assert!(rendered.contains("PIDs:"), "{rendered}");
    assert!(!rendered.contains("Image names:"), "{rendered}");
    assert!(!rendered.contains("terminates all"), "{rendered}");
}

#[test]
fn process_kill_confirmation_uses_enter_and_escape_only() {
    let mut confirm = make_test_app(1, 10);
    confirm.show_process_kill_confirmation = true;
    confirm
        .on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!confirm.show_process_kill_confirmation);
    assert_eq!(confirm.status, "No process PIDs selected");

    let mut cancel = make_test_app(1, 10);
    cancel.show_process_kill_confirmation = true;
    cancel
        .on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!cancel.show_process_kill_confirmation);
    assert_eq!(cancel.status, "Process kill canceled");

    for key in ['y', 'n'] {
        let mut ignored = make_test_app(1, 10);
        ignored.show_process_kill_confirmation = true;
        ignored
            .on_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE))
            .unwrap();
        assert!(ignored.show_process_kill_confirmation);
    }
}

#[test]
fn process_navigation_clamps_after_refresh_shrink() {
    let mut app = make_test_app(30, 10);
    app.select_last_row();
    app.snapshot.processes.truncate(5);
    app.snapshot.process_count = 5;
    app.rebuild_visible_process_cache();

    app.clamp_process_table_state();

    assert_eq!(app.process_table_state.selected(), Some(4));
    assert_eq!(app.process_table_state.offset(), 0);
}

#[test]
fn process_filter_matches_names_incrementally() {
    let mut app = make_test_app(3, 10);
    app.snapshot.processes[0].name = "cargo.exe".to_string();
    app.snapshot.processes[1].name = "winproc-tui.exe".to_string();
    app.snapshot.processes[2].name = "CARGO-watch.exe".to_string();

    app.begin_filter_edit();
    app.push_filter_char('c');
    app.push_filter_char('a');
    app.push_filter_char('r');

    let visible = app
        .visible_processes()
        .into_iter()
        .map(|process| process.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(visible, vec!["cargo.exe", "CARGO-watch.exe"]);
}

#[test]
fn process_filter_matches_paths_only_when_full_path_column_is_selected() {
    let mut app = make_test_app(2, 10);
    app.snapshot.processes[0].name = "app.exe".to_string();
    app.snapshot.processes[0].executable_path = Some(r"C:\work\alpha\app.exe".to_string());
    app.snapshot.processes[1].name = "app.exe".to_string();
    app.snapshot.processes[1].executable_path = Some(r"C:\work\beta\app.exe".to_string());

    app.begin_filter_edit();
    app.push_filter_char('b');
    app.push_filter_char('e');
    app.push_filter_char('t');
    app.push_filter_char('a');

    assert!(app.visible_processes().is_empty());

    app.process_columns.push(MetricColumn::FullPath);
    app.rebuild_visible_process_cache();

    let visible = app
        .visible_processes()
        .into_iter()
        .map(|process| process.executable_path.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(visible, vec![Some(r"C:\work\beta\app.exe")]);
}

#[test]
fn column_picker_full_path_toggle_rebuilds_active_filter_matches() {
    let mut app = make_test_app(2, 10);
    app.snapshot.processes[0].name = "app.exe".to_string();
    app.snapshot.processes[0].executable_path = Some(r"C:\work\alpha\app.exe".to_string());
    app.snapshot.processes[1].name = "app.exe".to_string();
    app.snapshot.processes[1].executable_path = Some(r"C:\work\beta\app.exe".to_string());

    app.begin_filter_edit();
    for ch in "beta".chars() {
        app.push_filter_char(ch);
    }
    assert!(app.visible_processes().is_empty());

    app.column_picker_index = MetricColumn::ALL
        .iter()
        .position(|column| *column == MetricColumn::FullPath)
        .unwrap();
    app.toggle_picker_column();

    let visible = app
        .visible_processes()
        .into_iter()
        .map(|process| process.executable_path.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(visible, vec![Some(r"C:\work\beta\app.exe")]);
}

#[test]
fn visible_process_window_returns_only_requested_rows() {
    let app = make_test_app(10, 10);

    let rows = app
        .visible_process_window(3, 4)
        .into_iter()
        .map(|(index, process)| (index, process.pid))
        .collect::<Vec<_>>();

    assert_eq!(rows, vec![(3, 3), (4, 4), (5, 5), (6, 6)]);
}

#[test]
fn process_filter_clamps_selection_to_visible_rows() {
    let mut app = make_test_app(4, 10);
    app.snapshot.processes[0].name = "alpha.exe".to_string();
    app.snapshot.processes[1].name = "beta.exe".to_string();
    app.snapshot.processes[2].name = "gamma.exe".to_string();
    app.snapshot.processes[3].name = "delta.exe".to_string();
    app.select_last_row();

    app.begin_filter_edit();
    app.push_filter_char('a');
    app.push_filter_char('l');

    assert_eq!(app.visible_process_count(), 1);
    assert_eq!(app.process_table_state.selected(), Some(0));
    assert_eq!(app.process_table_state.offset(), 0);
}

#[test]
fn process_filter_editing_blocks_row_navigation_keys() {
    let mut app = make_test_app(20, 5);
    app.select_process_index(7);

    app.begin_filter_edit();
    for key in [
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Home,
        KeyCode::End,
    ] {
        app.on_key(KeyEvent::new(key, KeyModifiers::NONE)).unwrap();
    }

    assert!(app.filter_editing);
    assert_eq!(app.filter_draft, "");
    assert_eq!(app.process_table_state.selected(), Some(7));
}

#[test]
fn filter_editing_space_edits_the_filter_instead_of_tracking() {
    let mut app = make_test_app(3, 10);
    app.snapshot.processes[0].name = "alpha.exe".to_string();
    app.snapshot.processes[1].name = "beta.exe".to_string();
    app.snapshot.processes[2].name = "gamma.exe".to_string();
    app.select_process_index(1);

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
        .unwrap();

    assert!(app.filter_editing);
    assert_eq!(app.filter_draft, "b ");
    assert!(app.watch_list.is_empty());
}

#[test]
fn filter_text_is_committed_by_up_or_down_then_selection_moves() {
    let cases = [(KeyCode::Up, 1, 0), (KeyCode::Down, 1, 2)];
    for (key, initial_selection, expected_selection) in cases {
        let mut app = make_test_app(3, 10);
        app.select_process_index(initial_selection);

        app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
            .unwrap();
        app.on_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
            .unwrap();
        app.on_key(KeyEvent::new(key, KeyModifiers::NONE)).unwrap();

        assert!(!app.filter_editing);
        assert_eq!(app.filter_text, "p");
        assert_eq!(app.filter_draft, "");
        assert_eq!(app.process_table_state.selected(), Some(expected_selection));
        assert_eq!(app.status, "Filter applied: p");
    }
}

#[test]
fn ordinary_character_does_not_start_filter_editing() {
    let mut app = make_test_app(3, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))
        .unwrap();

    assert!(!app.filter_editing);
    assert_eq!(app.filter_draft, "");
}

#[test]
fn f2_does_not_switch_the_application_theme() {
    let mut app = make_test_app(3, 10);
    let initial_theme_index = app.theme_index;

    app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.theme_index, initial_theme_index);
    assert!(app.status.is_empty());
}

#[test]
fn f12_cycles_color_schemes_and_wraps() {
    let mut app = make_test_app(3, 10);

    for expected in ["Yellow", "Orange", "Cyan", "Green"] {
        app.on_key(KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(app.theme().name, expected);
        assert_eq!(app.status, format!("Color scheme: {expected}"));
    }
}

#[test]
fn f12_switches_color_scheme_without_closing_help() {
    let mut app = make_test_app(3, 10);
    app.show_help = true;

    app.on_key(KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.theme().name, "Yellow");
    assert!(app.show_help);
}

#[test]
fn ctrl_f_starts_filter_editing() {
    let mut app = make_test_app(3, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();

    assert!(app.filter_editing);
    assert_eq!(app.filter_draft, "");
}

#[test]
fn ctrl_f_only_starts_filter_editing_when_processes_are_focused() {
    let mut app = make_test_app(3, 10);
    app.focused_panel = FocusedPanel::DetailsGraph;

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();

    assert!(!app.filter_editing);
    assert_eq!(app.filter_draft, "");
}

#[test]
fn ctrl_i_starts_process_jump_editing() {
    let mut app = make_test_app(3, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL))
        .unwrap();

    assert!(app.jump_editing);
    assert_eq!(app.jump_draft, "");
    assert_eq!(app.focused_panel, FocusedPanel::Processes);
    assert!(!app.show_system_info_dialog);
}

#[test]
fn process_jump_typing_moves_selection_without_filtering_rows() {
    let mut app = make_test_app(4, 10);
    app.snapshot.processes[0].name = "alpha.exe".to_string();
    app.snapshot.processes[1].name = "beta.exe".to_string();
    app.snapshot.processes[2].name = "alphabet.exe".to_string();
    app.snapshot.processes[3].name = "gamma.exe".to_string();

    app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.visible_process_count(), 4);
    assert_eq!(app.process_table_state.selected(), Some(0));
    assert_eq!(app.selected_visible_process().unwrap().name, "alpha.exe");

    app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(app.process_table_state.selected(), Some(2));
    assert_eq!(app.selected_visible_process().unwrap().name, "alphabet.exe");

    app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(app.process_table_state.selected(), Some(0));

    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.jump_editing);
    assert_eq!(app.jump_draft, "");
}

#[test]
fn ctrl_j_starts_process_jump_and_moves_to_next_match() {
    let mut app = make_test_app(4, 10);
    app.snapshot.processes[0].name = "winproc-tui.exe".to_string();
    app.snapshot.processes[1].name = "codex.exe".to_string();
    app.snapshot.processes[2].name = "win-helper.exe".to_string();
    app.snapshot.processes[3].name = "other.exe".to_string();

    app.on_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
        .unwrap();

    assert!(app.jump_editing);
    assert_eq!(app.process_table_state.selected(), Some(0));

    app.on_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
        .unwrap();

    assert_eq!(app.process_table_state.selected(), Some(2));
    assert_eq!(
        app.selected_visible_process().unwrap().name,
        "win-helper.exe"
    );

    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.jump_editing);
}

#[test]
fn process_jump_up_down_exits_jump_and_moves_selection() {
    let cases = [(KeyCode::Up, 2, 1), (KeyCode::Down, 1, 2)];

    for (key, start, expected) in cases {
        let mut app = make_test_app(4, 10);
        app.process_table_state.select(Some(start));

        app.on_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
            .unwrap();
        app.on_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
            .unwrap();
        assert!(app.jump_editing);

        app.on_key(KeyEvent::new(key, KeyModifiers::NONE)).unwrap();

        assert!(!app.jump_editing);
        assert_eq!(app.jump_draft, "");
        assert_eq!(app.process_table_state.selected(), Some(expected));
    }
}

#[test]
fn slash_does_not_start_process_jump() {
    let mut app = make_test_app(2, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE))
        .unwrap();

    assert!(!app.jump_editing);
    assert_eq!(app.process_table_state.selected(), Some(0));
}

#[test]
fn process_jump_title_shows_inline_query() {
    let mut app = make_test_app(3, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE))
        .unwrap();

    let rendered = render_app_to_text(&app, 100, 45);
    assert!(rendered.contains("Jump c_"), "{rendered}");
}

#[test]
fn process_jump_highlights_matching_name_text() {
    let mut app = make_test_app(2, 10);
    app.snapshot.processes[0].name = "winproc-tui.exe".to_string();
    app.snapshot.processes[1].name = "codex.exe".to_string();

    app.on_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
        .unwrap();

    let buffer = render_app_to_buffer(&app, 100, 45);
    let (x, y) = find_text_position(&buffer, "winproc-tui.exe")
        .expect("jump target name should be rendered");

    assert_eq!(buffer[(x, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(x + 1, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(x + 2, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(x + 3, y)].fg, ui::THEMES[0].text);
}

#[test]
fn process_filter_highlights_matching_name_text() {
    let mut app = make_test_app(2, 10);
    app.snapshot.processes[0].name = "winproc-tui.exe".to_string();
    app.snapshot.processes[1].name = "codex.exe".to_string();

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    for ch in "win".chars() {
        app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            .unwrap();
    }
    app.process_table_state.select(None);

    let buffer = render_app_to_buffer(&app, 100, 45);
    let (x, y) = find_text_position(&buffer, "winproc-tui.exe")
        .expect("filter target name should be rendered");

    assert_eq!(buffer[(x, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(x + 1, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(x + 2, y)].fg, ui::THEMES[0].warning);
    assert!(!buffer[(x, y)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(x + 3, y)].fg, ui::THEMES[0].text);
}

#[test]
fn process_filter_highlights_matching_full_path_text() {
    let mut app = make_test_app(2, 10);
    app.process_columns = vec![MetricColumn::FullPath];
    app.snapshot.processes[0].name = "app.exe".to_string();
    app.snapshot.processes[0].executable_path = Some(r"C:\work\alpha\app.exe".to_string());
    app.snapshot.processes[1].name = "app.exe".to_string();
    app.snapshot.processes[1].executable_path = Some(r"C:\work\beta\app.exe".to_string());

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    for ch in "beta".chars() {
        app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            .unwrap();
    }

    let buffer = render_app_to_buffer(&app, 160, 45);
    let path = r"C:\work\beta\app.exe";
    let (x, y) = find_text_position(&buffer, path).expect("filter target path should be rendered");
    let beta_x = x + r"C:\work\".chars().count() as u16;

    assert_eq!(buffer[(beta_x, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(beta_x + 1, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(beta_x + 2, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(beta_x + 3, y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(beta_x + 4, y)].fg, ui::THEMES[0].text);
}

#[test]
fn truncated_full_path_keeps_raw_filter_match_and_highlights_visible_tail() {
    let mut app = make_test_app(1, 10);
    app.process_columns = vec![MetricColumn::FullPath];
    app.process_column_widths.set(SortColumn::ProcessName, 8);
    app.snapshot.processes[0].name = "app.exe".to_string();
    let raw_path = format!(r"C:\{}\beta\app.exe", "hidden".repeat(16));
    app.snapshot.processes[0].executable_path = Some(raw_path.clone());

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    for ch in "beta".chars() {
        app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            .unwrap();
    }

    let buffer = render_app_to_buffer(&app, 80, 30);
    let (beta_x, y) = find_text_position(&buffer, r"beta\app.exe")
        .expect("the retained Full Path tail should be rendered");

    assert_eq!(app.visible_process_count(), 1);
    assert_eq!(
        app.snapshot.processes[0].executable_path.as_deref(),
        Some(raw_path.as_str())
    );
    assert!((0..beta_x).any(|x| buffer[(x, y)].symbol() == "⋯"));
    for offset in 0.."beta".len() as u16 {
        assert_eq!(buffer[(beta_x + offset, y)].fg, ui::THEMES[0].warning);
    }
}

#[test]
fn process_filter_does_not_duplicate_name_match_in_full_path() {
    let mut app = make_test_app(1, 10);
    app.process_columns = vec![MetricColumn::FullPath];
    app.snapshot.processes[0].name = "app.exe".to_string();
    app.snapshot.processes[0].executable_path = Some(r"C:\work\app.exe".to_string());

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    for ch in "app".chars() {
        app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            .unwrap();
    }

    let buffer = render_app_to_buffer(&app, 160, 45);
    let (name_x, name_y) =
        find_text_position(&buffer, "app.exe").expect("process name should be rendered");
    let path = r"C:\work\app.exe";
    let (path_x, path_y) = find_text_position(&buffer, path).expect("full path should be rendered");
    let path_match_x = path_x + r"C:\work\".chars().count() as u16;

    assert_eq!(buffer[(name_x, name_y)].fg, ui::THEMES[0].warning);
    assert_eq!(buffer[(path_match_x, path_y)].fg, ui::THEMES[0].text);
}

#[test]
fn filter_text_is_committed_by_enter() {
    let mut app = make_test_app(3, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert!(!app.filter_editing);
    assert_eq!(app.filter_text, "c");
}

#[test]
fn esc_clears_filter_and_exits_filter_editing() {
    let mut app = make_test_app(3, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.filter_text, "");
    assert!(!app.filter_editing);
    assert_eq!(app.filter_draft, "");
    assert_eq!(app.visible_process_count(), 3);
    assert_eq!(app.status, "Filter cleared");
}

#[test]
fn esc_clears_existing_filter_from_filter_editing() {
    let mut app = make_test_app(3, 10);
    app.filter_text = "proc".to_string();
    app.rebuild_visible_process_cache();
    app.clamp_process_table_state();

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();

    assert!(!app.filter_editing);
    assert_eq!(app.filter_text, "");
    assert_eq!(app.filter_draft, "");
    assert_eq!(app.visible_process_count(), 3);
    assert_eq!(app.status, "Filter cleared");
}

#[test]
fn details_toggle_changes_visibility_without_resetting_graph_workspace_state() {
    let mut app = make_test_app(3, 10);
    assign_private_graph(&mut app);
    add_test_graph(&mut app, 1);
    app.ab_comparison = Some(app::AbComparison { a: None, b: None });
    app.details_sample_selected = 7;
    app.details_sample_offset = 3;
    app.details_live = false;
    app.graph_scroll_row = 1;
    app.graph_time_span_seconds = 240;
    app.graph_time_offset_seconds = 30;
    let entries = app.graph_entries.clone();
    let active = app.active_graph_id;
    let comparison = app.ab_comparison.clone();
    assert!(app.show_details);

    app.on_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
        .unwrap();
    assert!(!app.show_details);

    app.on_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.show_details);
    assert_eq!(app.graph_entries, entries);
    assert_eq!(app.active_graph_id, active);
    assert_eq!(app.ab_comparison, comparison);
    assert_eq!(app.details_sample_selected, 7);
    assert_eq!(app.details_sample_offset, 3);
    assert!(!app.details_live);
    assert_eq!(app.graph_scroll_row, 1);
    assert_eq!(app.graph_time_span_seconds, 240);
    assert_eq!(app.graph_time_offset_seconds, 30);
}

#[test]
fn process_panel_shrinks_with_graphs_and_restores_full_height_when_hidden() {
    let mut app = make_test_app(2, 10);
    assign_private_graph(&mut app);
    let screen = Rect::new(0, 0, 120, 60);

    app::sync_layout_state(&mut app, screen);
    let shown = main_panel_areas_for_app(screen, &app);
    assert_eq!(shown.processes.area.height, 5);
    assert_eq!(shown.processes.page_size, 2);
    assert_eq!(shown.details.unwrap().y, shown.processes.area.bottom());

    app.on_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
        .unwrap();
    app::sync_layout_state(&mut app, screen);
    let hidden = main_panel_areas_for_app(screen, &app);
    assert!(hidden.processes.area.height > shown.processes.area.height);
    assert!(hidden.details.is_none());

    app.on_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
        .unwrap();
    app::sync_layout_state(&mut app, screen);
    assert_eq!(
        main_panel_areas_for_app(screen, &app).processes,
        shown.processes
    );
}

#[test]
fn dynamic_process_page_size_preserves_selection_and_clamps_offset() {
    let mut app = make_test_app(20, 10);
    assign_private_graph(&mut app);
    let screen = Rect::new(0, 0, 120, 60);
    app::sync_layout_state(&mut app, screen);
    app.select_process_index(15);
    app.ensure_selected_row_visible();
    assert_eq!(app.process_table_state.offset(), 6);

    app.filter_text = "proc-15".to_string();
    app.rebuild_visible_process_cache();
    app::sync_layout_state(&mut app, screen);

    assert_eq!(app.process_page_size, 1);
    assert_eq!(app.process_table_state.offset(), 0);
    assert_eq!(app.selected_visible_process().unwrap().name, "proc-15");

    app.filter_text.clear();
    app.rebuild_visible_process_cache();
    app::sync_layout_state(&mut app, screen);

    assert_eq!(app.process_page_size, 10);
    assert_eq!(app.process_table_state.offset(), 6);
    assert_eq!(app.selected_visible_process().unwrap().name, "proc-15");
}

#[test]
fn dynamic_graph_and_samples_regions_recompute_on_resize() {
    let mut app = make_test_app(2, 10);
    assign_private_graph(&mut app);
    let short_screen = Rect::new(0, 0, 120, 45);
    let tall_screen = Rect::new(0, 0, 120, 60);

    app::sync_layout_state(&mut app, short_screen);
    let short = main_panel_areas_for_app(short_screen, &app);
    let short_sample_page_size = app.details_sample_page_size;

    app::sync_layout_state(&mut app, tall_screen);
    let tall = main_panel_areas_for_app(tall_screen, &app);

    assert_eq!(short.processes.area.height, tall.processes.area.height);
    assert_eq!(
        tall.details.unwrap().height - short.details.unwrap().height,
        15
    );
    assert_eq!(app.details_sample_page_size - short_sample_page_size, 15);
}

#[test]
fn tracked_total_is_rendered_when_it_is_the_only_process_row() {
    let mut app = make_test_app(1, 10);
    app.snapshot.processes[0].name = "target.exe".to_string();
    app.rebuild_visible_process_cache();
    app.clamp_process_table_state();
    assign_private_graph(&mut app);
    track_process_name(&mut app, "target.exe");
    app.filter_text = "missing".to_string();
    app.rebuild_visible_process_cache();
    let screen = Rect::new(0, 0, 120, 45);

    app::sync_layout_state(&mut app, screen);
    let panels = main_panel_areas_for_app(screen, &app);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let (_, total_y) =
        find_text_position(&buffer, "Tracked Total").expect("Tracked Total should render");

    assert_eq!(app.visible_process_count(), 0);
    assert_eq!(panels.processes.area.height, 4);
    assert_eq!(panels.processes.page_size, 0);
    assert!(panels.processes.show_tracked_total);
    assert_eq!(total_y, panels.processes.area.y + 2);
}

#[test]
fn mouse_selection_uses_dynamic_process_graph_boundary() {
    let mut app = make_test_app(3, 10);
    assign_private_graph(&mut app);
    let screen = Rect::new(0, 0, 120, 45);
    app::sync_layout_state(&mut app, screen);
    let panels = main_panel_areas_for_app(screen, &app);
    let process_area = panels.processes.area;

    app.on_mouse(
        left_click(process_area.x + 4, process_area.bottom() - 2),
        screen,
    );
    assert_eq!(app.process_table_state.selected(), Some(2));

    let graph = details_graph_area_for_app(screen, &app).unwrap();
    app.on_mouse(left_click(graph.x + 1, graph.y + 1), screen);

    assert_eq!(app.focused_panel, FocusedPanel::DetailsGraph);
    assert_eq!(app.process_table_state.selected(), Some(2));
}

#[test]
fn g_without_graph_metrics_shows_warning_dialog() {
    let mut app = make_test_app(3, 10);

    app.on_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
        .unwrap();

    assert!(!app.show_details);
    assert!(app.show_no_graph_metrics_warning);
    assert_eq!(app.status, "No metric is selected for graphing.");

    let rendered = render_app_to_text(&app, 100, 45);
    assert!(
        rendered.contains("No metric is selected for graphing."),
        "{rendered}"
    );
    assert!(
        rendered.contains("Select a metric, then press Space or double-click it."),
        "{rendered}"
    );
    assert!(rendered.contains("Enter/Esc Close"), "{rendered}");

    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.show_no_graph_metrics_warning);
}

#[test]
fn source_number_keys_only_show_graph_migration_guidance() {
    let mut app = make_test_app(3, 10);
    app.set_screen_area(Rect::new(0, 0, 120, 80));

    for key in ['1', '2', '3', '4'] {
        app.on_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE))
            .unwrap();
        assert!(app.graph_entries.is_empty());
        assert_eq!(app.status, "Use Space or double-click to graph this metric");
    }
    app.on_key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.graph_entries.is_empty());
    assert_eq!(app.status, "Remove Graphs with Delete or the remove button");
}

#[test]
fn delete_on_live_process_opens_kill_confirm_before_graph_clear() {
    let mut app = make_test_app(3, 10);
    app.set_screen_area(Rect::new(0, 0, 120, 80));
    assign_private_graph(&mut app);
    app.focused_panel = FocusedPanel::Processes;
    app.on_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))
        .unwrap();

    assert!(app.show_process_kill_confirmation);
    assert_eq!(app.graph_entries.len(), 1);
    assert_eq!(app.process_kill_targets.len(), 1);
}

#[test]
fn space_on_pid_and_process_columns_toggles_tracking() {
    let mut app = make_test_app(3, 10);
    let selected_name = app.snapshot.processes[0].name.clone();

    app.selected_process_column_index = 0;
    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.watch_list, vec![selected_name]);
    assert!(app.graph_entries.is_empty());
    assert!(!app.show_details);

    app.selected_process_column_index = 1;
    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
        .unwrap();

    assert!(app.watch_list.is_empty());
    assert!(app.graph_entries.is_empty());
    assert!(!app.show_details);
    assert!(!app.show_metric_column_warning);
    assert!(app.status.starts_with("Removed from Tracking List:"));
}

#[test]
fn tab_leaves_graph_workspace_when_samples_are_hidden() {
    let mut app = make_test_app(1, 10);
    let identity = app.selected_visible_process_identity().unwrap();
    app.add_or_reveal_graph_source(
        GraphSlot::process(identity.clone(), DetailsMetric::Private),
        FocusedPanel::Processes,
    );
    app.add_or_reveal_graph_source(
        GraphSlot::process(identity, DetailsMetric::Workset),
        FocusedPanel::Processes,
    );
    app.show_samples_panel = false;
    app.focused_panel = FocusedPanel::DetailsGraph;
    let active_id = app.active_graph_id;

    app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.focused_panel, FocusedPanel::System);
    assert_eq!(app.resource_panel, app::ResourcePanel::Memory);
    assert_eq!(app.active_graph_id, active_id);
}

#[test]
fn process_navigation_only_runs_when_processes_are_focused() {
    let mut app = make_test_app(3, 10);
    app.focused_panel = FocusedPanel::System;

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.process_table_state.selected(), Some(0));
}

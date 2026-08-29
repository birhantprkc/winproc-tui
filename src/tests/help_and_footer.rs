use super::support::{
    assign_private_graph, find_symbol_position, find_text_position, find_text_position_in_area,
    make_test_app, render_app_to_buffer, render_app_to_text,
};
use crate::app::FocusedPanel;
use crate::ui;
use crate::ui::{help_area, help_scrollbar_area, main_panel_areas_for_app};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

#[test]
fn help_opens_with_f1_or_question_mark() {
    for key in [
        KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
    ] {
        let mut app = make_test_app(1, 10);

        app.on_key(key).unwrap();

        assert!(app.show_help);
    }
}

#[test]
fn help_closes_with_escape_enter_f1_or_question_mark() {
    for key in [
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
    ] {
        let mut app = make_test_app(1, 10);
        app.show_help = true;

        app.on_key(key).unwrap();

        assert!(!app.show_help);
        assert!(!app.show_quit_confirmation);
    }
}

#[test]
fn help_blocks_normal_shortcuts_while_open() {
    let mut app = make_test_app(1, 10);
    app.show_help = true;

    app.on_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
        .unwrap();

    assert!(app.show_help);
    assert!(!app.show_quit_confirmation);
}

#[test]
fn help_dialog_buffer_shows_two_column_layout() {
    let mut app = make_test_app(3, 10);
    app.show_help = true;

    let rendered = render_app_to_text(&app, 150, 70);
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered.contains(&format!(
            "winproc-tui {} · Keyboard shortcuts",
            env!("CARGO_PKG_VERSION")
        )),
        "{rendered}"
    );
    assert!(rendered.contains("Keyboard shortcuts"), "{rendered}");
    assert!(
        rendered.contains("History: 120/7,200 normal/tracked"),
        "{rendered}"
    );
    assert!(rendered.contains("Global  (any focus)"), "{rendered}");
    assert!(rendered.contains("Processes"), "{rendered}");
    assert!(rendered.contains("Toggle Flat / Tree view"), "{rendered}");
    assert!(
        rendered.contains("Expand/collapse Tree row (no filter)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Expand/collapse subtree (no filter)"),
        "{rendered}"
    );
    assert!(rendered.contains("MEM/GPU"), "{rendered}");
    assert!(rendered.contains("NW/DISK"), "{rendered}");
    assert!(
        rendered.contains("Graph Workspace  (Graph focus)"),
        "{rendered}"
    );
    assert!(rendered.contains("Samples"), "{rendered}");
    assert!(
        rendered.contains("Tracking  (Processes focus)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("A/B comparison  (Graph or Samples)"),
        "{rendered}"
    );
    assert!(rendered.contains("Mouse"), "{rendered}");
    assert!(rendered.contains("Processes / Graph split"), "{rendered}");
    assert!(!rendered.contains("▋"), "{rendered}");

    assert!(rendered.contains("Set A range endpoint"), "{rendered}");
    assert!(
        rendered.contains("Set B; show range statistics"),
        "{rendered}"
    );
    assert!(rendered.contains("Jump to A or B"), "{rendered}");
    assert!(rendered.contains("Clear A/B comparison"), "{rendered}");

    assert!(rendered.contains("Pan time range"), "{rendered}");
    assert!(
        rendered.contains("Select previous Graph slot"),
        "{rendered}"
    );
    assert!(rendered.contains("Select next Graph slot"), "{rendered}");
    assert!(rendered.contains("Select older sample"), "{rendered}");
    assert!(rendered.contains("Select newer sample"), "{rendered}");
    assert!(rendered.contains("Remove active Graph"), "{rendered}");
    assert!(
        rendered.contains("Toggle active Graph Raw / MA5"),
        "{rendered}"
    );
    assert!(
        rendered.contains("f/z") && rendered.contains("Fit all / compact Min0"),
        "{rendered}"
    );
    assert!(
        rendered.contains("v/d/l") && rendered.contains("Samples / Delta / layout"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Start recording / confirm stop"),
        "{rendered}"
    );
    assert!(rendered.contains("Pause / Resume"), "{rendered}");
    assert!(rendered.contains("Copy selected row"), "{rendered}");
    assert!(!rendered.contains("Open Settings"), "{rendered}");

    assert!(rendered.contains("Select row range"), "{rendered}");
    assert!(rendered.contains("Toggle row selection"), "{rendered}");
    assert!(
        rendered.contains("Kill selected live process"),
        "{rendered}"
    );
    assert!(rendered.contains("Info/detail / Files"), "{rendered}");
    assert!(rendered.contains("Switch Info tabs"), "{rendered}");
    assert!(rendered.contains("Refresh Info tab"), "{rendered}");

    assert!(rendered.contains("Click panel"), "{rendered}");
    assert!(rendered.contains("Samples auto-scroll"), "{rendered}");
    assert!(rendered.contains("PageUp/PageDown"), "{rendered}");
    assert!(rendered.contains("Change time span"), "{rendered}");
    assert!(
        rendered.contains("Increase / decrease Processes height"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Reset Processes height to Auto"),
        "{rendered}"
    );
    assert!(rendered.contains("Resize Processes height"), "{rendered}");

    assert!(!rendered.contains("Details panel"), "{rendered}");
    assert!(!rendered.contains("Dialogs"), "{rendered}");
    assert!(!rendered.contains("Recording path"), "{rendered}");
    assert!(!rendered.contains("Sampling interval"), "{rendered}");
    assert!(
        !rendered.contains("Esc / Enter closes this help dialog."),
        "{rendered}"
    );
    assert!(!rendered.contains("F6"), "{rendered}");
    assert!(!rendered.contains("[ Close ]"), "{rendered}");
    assert!(
        rendered.contains("F1/?") && rendered.contains("Toggle Help"),
        "{rendered}"
    );
    assert!(rendered.contains("F12"), "{rendered}");
    assert!(rendered.contains("Cycle color scheme"), "{rendered}");
    assert!(rendered.contains("Esc/Enter/F1/? Close"), "{rendered}");
    assert!(rendered.contains("Footer: focused actions."), "{rendered}");
    assert!(
        rendered.contains("Scheme colors mark active items; T marks tracked."),
        "{rendered}"
    );
    assert!(!rendered_lower.contains("baseline"), "{rendered}");
}

#[test]
fn help_dialog_header_and_shortcuts_use_footer_like_styles() {
    let mut app = make_test_app(3, 10);
    app.show_help = true;

    let buffer = render_app_to_buffer(&app, 100, 45);
    let theme = ui::THEMES[0];

    let title = format!(
        "winproc-tui {} · Keyboard shortcuts",
        env!("CARGO_PKG_VERSION")
    );
    let (title_x, title_y) =
        find_text_position(&buffer, &title).expect("help dialog title should be rendered");
    assert_eq!(title_x, help_area(Rect::new(0, 0, 100, 45)).x + 2);
    let title_cell = &buffer[(title_x, title_y)];
    assert_eq!(title_cell.fg, theme.text);
    assert_ne!(title_cell.fg, theme.accent);
    assert!(title_cell.modifier.contains(ratatui::style::Modifier::BOLD));

    let (group_x, group_y) =
        find_text_position(&buffer, "Global").expect("group title should be rendered");
    let group_cell = &buffer[(group_x, group_y)];
    assert_eq!(group_cell.symbol(), "G");
    assert_eq!(group_cell.fg, theme.accent);
    assert!(group_cell.modifier.contains(ratatui::style::Modifier::BOLD));
    assert!(
        !group_cell
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED)
    );

    let (key_x, key_y) =
        find_text_position(&buffer, "Ctrl+F").expect("shortcut key should be rendered");
    let key_cell = &buffer[(key_x, key_y)];
    assert_eq!(key_cell.fg, theme.key_hint);
    assert_eq!(key_cell.bg, theme.panel);
    assert!(!key_cell.modifier.contains(ratatui::style::Modifier::BOLD));

    let label_cell = &buffer[(key_x + "Ctrl+F ".len() as u16, key_y)];
    assert_eq!(label_cell.fg, theme.text);
}

#[test]
fn help_dialog_panel_fits_rendered_content() {
    let screen = Rect::new(0, 0, 120, 50);
    let popup = help_area(screen);

    assert!(popup.width <= screen.width);
    assert!(popup.height <= screen.height);
    assert!(popup.width >= 50, "popup too narrow: {popup:?}");
    assert!(popup.height >= 25, "popup too short: {popup:?}");
}

#[test]
fn help_dialog_scrolls_when_content_overflows() {
    let mut app = make_test_app(3, 10);
    app.show_help = true;
    let screen = Rect::new(0, 0, 100, 20);

    let top_rendered = render_app_to_text(&app, screen.width, screen.height);
    let top_buffer = render_app_to_buffer(&app, screen.width, screen.height);

    assert!(
        top_rendered.contains("Keyboard shortcuts"),
        "{top_rendered}"
    );
    assert!(top_rendered.contains("Global"), "{top_rendered}");
    assert!(
        find_symbol_position(&top_buffer, "█").is_some(),
        "{top_rendered}"
    );

    app.set_help_page_size(ui::help_page_size_for_screen(screen));
    app.scroll_help_end();
    let bottom_rendered = render_app_to_text(&app, screen.width, screen.height);

    assert!(
        bottom_rendered.contains("Esc/Enter/F1/? Close"),
        "{bottom_rendered}"
    );
    assert!(
        !bottom_rendered.contains("Esc / Enter closes this help dialog."),
        "{bottom_rendered}"
    );
}

#[test]
fn help_dialog_keyboard_scroll_updates_offset() {
    let mut app = make_test_app(3, 10);
    app.show_help = true;
    app.set_help_page_size(ui::help_page_size_for_screen(Rect::new(0, 0, 100, 20)));

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.help_scroll.offset, 1);

    app.on_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))
        .unwrap();
    assert!(app.help_scroll.offset > 1);

    app.on_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.help_scroll.offset, 0);
}

#[test]
fn help_dialog_scrollbar_drag_scrolls_content() {
    let mut app = make_test_app(3, 10);
    app.show_help = true;
    let screen = Rect::new(0, 0, 100, 20);
    app.set_help_page_size(ui::help_page_size_for_screen(screen));
    let scrollbar = help_scrollbar_area(screen, app.help_scroll.page_size)
        .expect("small help dialog should have a scrollbar");

    app.on_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: scrollbar.x,
            row: scrollbar.y,
            modifiers: KeyModifiers::NONE,
        },
        screen,
    );
    assert!(app.help_scroll.dragging);
    assert_eq!(app.help_scroll.offset, 0);

    app.on_mouse(
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: scrollbar.x,
            row: scrollbar.bottom().saturating_sub(1),
            modifiers: KeyModifiers::NONE,
        },
        screen,
    );
    assert!(app.help_scroll.offset > 0);

    app.on_mouse(
        MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: scrollbar.x,
            row: scrollbar.bottom().saturating_sub(1),
            modifiers: KeyModifiers::NONE,
        },
        screen,
    );
    assert!(!app.help_scroll.dragging);
}

#[test]
fn footer_shows_process_context_on_one_row() {
    let mut app = make_test_app(3, 10);
    app.status = "Copied row: proc-0".to_string();

    let rendered = render_app_to_text(&app, 260, 30);

    assert!(rendered.contains("PROCESSES"), "{rendered}");
    assert!(rendered.contains("Ctrl+P Pause"), "{rendered}");
    assert!(rendered.contains("Ctrl+T Lists"), "{rendered}");
    assert!(rendered.contains("c Columns"), "{rendered}");
    assert!(rendered.contains("s Sort"), "{rendered}");
    assert!(rendered.contains("v Flat/Tree"), "{rendered}");
    assert!(!rendered.contains("e Expand/Collapse"), "{rendered}");
    assert!(rendered.contains("g Graphs"), "{rendered}");
    assert!(rendered.contains("Ctrl+I Jump"), "{rendered}");
    assert!(!rendered.contains("Shift+←/→ Move column"), "{rendered}");
    assert!(rendered.contains("Space Graph"), "{rendered}");
    assert!(rendered.contains("Enter/f Info/Files"), "{rendered}");
    assert!(rendered.contains("t Track"), "{rendered}");
    assert!(rendered.contains("Shift+T Tracked-only"), "{rendered}");
    assert!(rendered.contains("d Kill"), "{rendered}");
    assert!(rendered.contains("Ctrl+F Filter"), "{rendered}");
    assert!(rendered.contains("Esc Quit"), "{rendered}");
    assert!(!rendered.contains("Tab Focus"), "{rendered}");
    assert!(rendered.contains("F12 Color"), "{rendered}");
    assert!(rendered.contains("F1/? Help"), "{rendered}");
    assert!(!rendered.contains("Status  "), "{rendered}");
    assert!(!rendered.contains("Copied row: proc-0"), "{rendered}");
    assert!(!rendered.contains("Up/Down Row"), "{rendered}");
    assert!(!rendered.contains("Left/Right Column"), "{rendered}");
    assert!(!rendered.contains("Ctrl+R Record"), "{rendered}");
    assert!(!rendered.contains("Ctrl+O Settings"), "{rendered}");

    app.on_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
        .unwrap();
    let tree = render_app_to_text(&app, 260, 30);
    assert!(tree.contains("v Flat/Tree"), "{tree}");
    assert!(tree.contains("e Expand/Collapse"), "{tree}");

    app.filter_text = "proc".to_string();
    app.rebuild_visible_process_cache();
    let filtered_tree = render_app_to_text(&app, 260, 30);
    assert!(filtered_tree.contains("v Flat/Tree"), "{filtered_tree}");
    assert!(
        !filtered_tree.contains("e Expand/Collapse"),
        "{filtered_tree}"
    );
}

#[test]
fn footer_keeps_primary_action_visible_at_narrow_width() {
    let app = make_test_app(3, 10);
    let buffer = render_app_to_buffer(&app, 30, 24);
    let footer = Rect::new(0, 23, 30, 1);
    let (action_x, action_y) = find_text_position_in_area(&buffer, footer, "Space Graph")
        .expect("primary action should remain visible");

    assert_eq!(action_x, 0);
    assert_eq!(action_y, footer.y);
    assert!(find_text_position_in_area(&buffer, footer, "PROCESSES").is_none());
    assert!(find_text_position(&buffer, "Ctrl+T Lists").is_none());
}

#[test]
fn process_footer_labels_space_for_the_selected_cell_action() {
    let mut app = make_test_app(3, 10);
    app.selected_process_column_index = 0;

    let identity_column = render_app_to_text(&app, 170, 30);
    assert!(identity_column.contains("Space Track"), "{identity_column}");
    assert!(
        !identity_column.contains("Space Graph"),
        "{identity_column}"
    );

    app.selected_process_column_index = 2;
    let metric_column = render_app_to_text(&app, 170, 30);
    assert!(metric_column.contains("Space Graph"), "{metric_column}");
    assert!(!metric_column.contains("Space Track"), "{metric_column}");
}

#[test]
fn footer_shortcuts_follow_the_focused_panel() {
    let mut app = make_test_app(3, 10);
    app.focused_panel = FocusedPanel::System;
    let system = render_app_to_text(&app, 170, 30);
    assert!(system.contains("i System info"), "{system}");
    assert!(!system.contains("Up/Down Metric"), "{system}");
    assert!(!system.contains("Left/Right Column"), "{system}");

    assign_private_graph(&mut app);
    app.focused_panel = FocusedPanel::DetailsGraph;
    let graph = render_app_to_text(&app, 260, 45);
    assert!(graph.contains("↑/↓ Slot"), "{graph}");
    assert!(graph.contains("←/→ Sample"), "{graph}");
    assert!(!graph.contains("Prev Slot"), "{graph}");
    assert!(!graph.contains("Next Slot"), "{graph}");
    assert!(graph.contains("Del Remove Graph"), "{graph}");
    assert!(graph.contains("m Raw/MA5"), "{graph}");
    assert!(graph.contains("Enter Info"), "{graph}");
    assert!(graph.contains("Ctrl+←/→ Pan"), "{graph}");
    assert!(graph.contains("PgUp/PgDn Span"), "{graph}");
    assert!(graph.contains("f/z Fit/Min 0"), "{graph}");
    assert!(graph.contains("a/b Set A/B range"), "{graph}");
    assert!(graph.contains("Shift+A/B Jump A/B"), "{graph}");

    app.focused_panel = FocusedPanel::DetailsSamples;
    let samples = render_app_to_text(&app, 260, 45);
    assert!(samples.contains("↑/← Older"), "{samples}");
    assert!(samples.contains("↓/→ Newer"), "{samples}");
    assert!(samples.contains("Del Remove Graph"), "{samples}");
    assert!(samples.contains("m Raw/MA5"), "{samples}");
    assert!(samples.contains("PgUp/PgDn Scroll"), "{samples}");
    assert!(samples.contains("Home/End Edge"), "{samples}");
    assert!(samples.contains("f/z Fit/Min 0"), "{samples}");
    assert!(samples.contains("Shift+A/B Jump A/B"), "{samples}");
    assert!(samples.contains("a/b Set A/B range"), "{samples}");
    assert!(samples.contains("x Clear A/B"), "{samples}");
}

#[test]
fn footer_shows_process_height_shortcuts_only_for_visible_workspace_focus() {
    let mut app = make_test_app(3, 10);
    let hidden = render_app_to_text(&app, 360, 45);
    assert!(!hidden.contains("h/H/Alt+H Height"), "{hidden}");

    assign_private_graph(&mut app);
    for focused_panel in [
        FocusedPanel::Processes,
        FocusedPanel::DetailsGraph,
        FocusedPanel::DetailsSamples,
    ] {
        app.focused_panel = focused_panel;
        let rendered = render_app_to_text(&app, 360, 45);
        assert!(
            rendered.contains("h/H/Alt+H Height"),
            "{focused_panel:?}: {rendered}"
        );
    }

    app.focused_panel = FocusedPanel::System;
    let system = render_app_to_text(&app, 360, 45);
    assert!(!system.contains("h/H/Alt+H Height"), "{system}");
}

#[test]
fn footer_shows_pause_and_omits_tab_for_every_focused_panel() {
    let mut app = make_test_app(3, 10);

    for focused_panel in [
        FocusedPanel::System,
        FocusedPanel::SystemActivity,
        FocusedPanel::Cpu,
        FocusedPanel::Processes,
        FocusedPanel::DetailsGraph,
        FocusedPanel::DetailsSamples,
    ] {
        app.focused_panel = focused_panel;
        let rendered = render_app_to_text(&app, 420, 45);
        assert!(
            rendered.contains("Ctrl+P Pause"),
            "{focused_panel:?}: {rendered}"
        );
        assert!(
            !rendered.contains("Tab Focus"),
            "{focused_panel:?}: {rendered}"
        );
    }
}

#[test]
fn help_dialog_takes_focus_border_from_previous_panel() {
    let mut app = make_test_app(3, 10);
    app.focused_panel = FocusedPanel::Processes;
    app.show_help = true;

    let screen = Rect::new(0, 0, 140, 70);
    let popup = help_area(screen);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let process_table = main_panel_areas_for_app(screen, &app).processes.area;
    assert_eq!(buffer[(popup.x, popup.y)].fg, app.theme().focus_border);
    assert_eq!(
        buffer[(process_table.x, process_table.y)].fg,
        app.theme().border
    );
}

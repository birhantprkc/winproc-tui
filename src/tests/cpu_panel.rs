use super::support::{
    add_test_graph, buffer_to_text, find_text_position_in_area, left_click, make_test_app,
    mouse_move, render_app_to_buffer, render_app_to_text,
};
use crate::app;
use crate::app::{FocusedPanel, GraphSlot, GraphValueFormat};
use crate::model::{CpuCoreKind, CpuLogicalProcessorSample, SystemMetric};
use crate::ui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Modifier;

#[test]
fn cpu_panel_renders_compact_summary_and_per_core_control() {
    let mut app = make_test_app(3, 10);
    app.snapshot.cpu_total_usage_percent = Some(42);
    app.snapshot.cpu_user_usage_percent = Some(31);
    app.snapshot.cpu_kernel_usage_percent = Some(11);
    app.snapshot.cpu_p_core_frequency_mhz = Some(3_200);
    app.snapshot.cpu_e_core_frequency_mhz = Some(1_800);
    app.snapshot.thread_count = Some(4_335);
    app.snapshot.cpu_logical_processors = vec![
        CpuLogicalProcessorSample {
            usage_percent: 1,
            kind: Some(CpuCoreKind::Performance),
        },
        CpuLogicalProcessorSample {
            usage_percent: 22,
            kind: Some(CpuCoreKind::Performance),
        },
        CpuLogicalProcessorSample {
            usage_percent: 99,
            kind: Some(CpuCoreKind::Efficiency),
        },
    ];

    let rendered = render_app_to_text(&app, 120, 45);

    assert!(rendered.contains("CPU"), "{rendered}");
    assert!(
        rendered.contains("Usage       42% (U 31%, K 11%)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Freq(P/E)  3200 MHz / 1800 MHz"),
        "{rendered}"
    );
    assert!(rendered.contains("[Per-core Usage (P/E)]"), "{rendered}");
    assert!(rendered.contains("Threads    4,335"), "{rendered}");
    assert!(rendered.contains("Processes  3"), "{rendered}");
    assert!(!rendered.contains("P ▁▂"), "{rendered}");

    let screen = Rect::new(0, 0, 120, 45);
    let area = ui::cpu_panel_area_for_screen(screen, &app);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let (_, threads_y) = find_text_position_in_area(&buffer, area, "Threads").unwrap();
    let (_, processes_y) = find_text_position_in_area(&buffer, area, "Processes").unwrap();
    let (_, per_core_y) =
        find_text_position_in_area(&buffer, area, "[Per-core Usage (P/E)]").unwrap();
    assert!(threads_y < processes_y && processes_y < per_core_y);
}

#[test]
fn cpu_frequency_omits_e_segment_when_no_e_core_exists() {
    let mut app = make_test_app(3, 10);
    app.snapshot.cpu_p_core_frequency_mhz = Some(3_200);
    app.snapshot.cpu_e_core_frequency_mhz = Some(1_800);
    app.snapshot.cpu_logical_processors = vec![CpuLogicalProcessorSample {
        usage_percent: 25,
        kind: Some(CpuCoreKind::Performance),
    }];

    let rendered = render_app_to_text(&app, 120, 30);

    assert!(rendered.contains("Freq(P/E)  3200 MHz"), "{rendered}");
    assert!(!rendered.contains("3200 MHz /"), "{rendered}");
}

#[test]
fn cpu_panel_width_does_not_grow_with_logical_cpu_count() {
    let screen = Rect::new(0, 0, 180, 30);
    let mut small = make_test_app(3, 10);
    small.snapshot.cpu_logical_processors = vec![CpuLogicalProcessorSample {
        usage_percent: 25,
        kind: Some(CpuCoreKind::Performance),
    }];
    let mut large = make_test_app(3, 10);
    large.snapshot.cpu_logical_processors = vec![
        CpuLogicalProcessorSample {
            usage_percent: 25,
            kind: Some(CpuCoreKind::Performance),
        };
        128
    ];

    assert_eq!(
        ui::cpu_panel_area_for_screen(screen, &small).width,
        ui::cpu_panel_area_for_screen(screen, &large).width
    );
}

#[test]
fn cpu_per_core_control_hovers_and_opens_a_scrollable_dialog() {
    let screen = Rect::new(0, 0, 100, 15);
    for (theme_index, theme) in ui::THEMES.iter().copied().enumerate() {
        let mut app = make_test_app(3, 10);
        app.theme_index = theme_index;
        app.snapshot.cpu_logical_processors = (0..40)
            .map(|index| CpuLogicalProcessorSample {
                usage_percent: index as u8,
                kind: Some(if index % 2 == 0 {
                    CpuCoreKind::Performance
                } else {
                    CpuCoreKind::Efficiency
                }),
            })
            .collect();
        app.focused_panel = FocusedPanel::Cpu;
        app.on_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))
            .unwrap();
        assert!(app.cpu_per_core_selected());
        app.on_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
            .unwrap();
        assert_eq!(app.selected_cpu_metric(), Some(SystemMetric::ProcessCount));
        app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .unwrap();
        assert!(app.cpu_per_core_selected());
        let button = ui::cpu_per_core_button_area(ui::cpu_panel_area_for_screen(screen, &app))
            .expect("per-core button should render");

        let selected = render_app_to_buffer(&app, screen.width, screen.height);
        assert_eq!(
            selected[(button.x, button.y)].bg,
            theme.table_selection_surface
        );

        app.on_mouse(mouse_move(button.x, button.y), screen);
        let hovered = render_app_to_buffer(&app, screen.width, screen.height);
        assert_eq!(hovered[(button.x, button.y)].bg, theme.focus_surface);
        assert!(
            hovered[(button.x, button.y)]
                .modifier
                .contains(Modifier::BOLD)
        );

        app.on_mouse(mouse_move(0, screen.height - 1), screen);
        assert!(!app.cpu_per_core_hovered);
        let unhovered = render_app_to_buffer(&app, screen.width, screen.height);
        assert_eq!(
            unhovered[(button.x, button.y)].bg,
            theme.table_selection_surface
        );

        app.on_mouse(left_click(button.x, button.y), screen);
        assert!(app.show_cpu_core_dialog);
        assert_eq!(app.focused_panel, FocusedPanel::Cpu);
        assert!(app.graph_entries.is_empty());
        app::sync_layout_state(&mut app, screen);

        app.on_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))
            .unwrap();
        assert!(app.cpu_core_scroll.offset > 0);
        let dialog = render_app_to_text(&app, screen.width, screen.height);
        assert!(dialog.contains("PER-CORE CPU USAGE"), "{dialog}");
        assert!(dialog.contains("CPU 39 (E)"), "{dialog}");
        assert!(dialog.contains("Enter/Esc Close"), "{dialog}");

        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(!app.show_cpu_core_dialog);
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .unwrap();
        assert!(app.show_cpu_core_dialog);
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(!app.show_cpu_core_dialog);
    }
}

#[test]
fn cpu_panel_space_assigns_cpu_average_graph() {
    let mut app = make_test_app(3, 10);
    app.focused_panel = FocusedPanel::Cpu;
    app.snapshot.cpu_total_usage_percent = Some(42);
    app.system_history.record_snapshot(&app.snapshot);

    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(
        app.active_graph_slot(),
        Some(&GraphSlot::system(SystemMetric::CpuAverage))
    );
    assert!(app.show_details);
    assert_eq!(
        app.active_graph_slot().map(GraphSlot::value_format),
        Some(GraphValueFormat::Percent)
    );
    assert_eq!(
        app.graph_slot_samples(app.active_graph_slot().unwrap())[0].value,
        Some(42.0)
    );

    let screen = Rect::new(0, 0, 180, 45);
    let area = ui::cpu_panel_area_for_screen(screen, &app);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let rendered = buffer_to_text(&buffer);
    assert!(rendered.contains("Usage       42%"), "{rendered}");
    assert!(!rendered.contains("1  Usage"), "{rendered}");
    assert!(rendered.contains("Slot#1 · CPU Usage"), "{rendered}");

    let (x, y) = find_text_position_in_area(&buffer, area, "42%")
        .expect("registered CPU value should render");
    let value = &buffer[(x, y)];
    assert_eq!(value.fg, app.theme().active_series);
    assert!(value.modifier.contains(Modifier::BOLD));
}

#[test]
fn cpu_panel_can_select_graph_and_copy_threads() {
    let mut app = make_test_app(3, 10);
    app.focused_panel = FocusedPanel::Cpu;
    app.snapshot.thread_count = Some(4_335);
    app.system_history.record_snapshot(&app.snapshot);

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.selected_cpu_metric(), Some(SystemMetric::ThreadCount));
    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(
        app.active_graph_slot(),
        Some(&GraphSlot::system(SystemMetric::ThreadCount))
    );
    assert_eq!(
        app.graph_slot_samples(app.active_graph_slot().unwrap())[0].value,
        Some(4_335.0)
    );

    app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(
        app::clipboard::last_copied_text().as_deref(),
        Some("Threads\t4,335")
    );
}

#[test]
fn cpu_panel_can_select_graph_and_copy_processes() {
    let mut app = make_test_app(214, 10);
    app.focused_panel = FocusedPanel::Cpu;
    app.system_history.record_snapshot(&app.snapshot);

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.selected_cpu_metric(), Some(SystemMetric::ProcessCount));

    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
        .unwrap();
    assert_eq!(
        app.active_graph_slot(),
        Some(&GraphSlot::system(SystemMetric::ProcessCount))
    );
    assert_eq!(
        app.active_graph_slot().map(GraphSlot::value_format),
        Some(GraphValueFormat::Count)
    );
    assert_eq!(
        app.graph_slot_samples(app.active_graph_slot().unwrap())[0].value,
        Some(214.0)
    );

    let screen = Rect::new(0, 0, 180, 45);
    let area = ui::cpu_panel_area_for_screen(screen, &app);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let metric_view = buffer_to_text(&buffer);
    assert!(metric_view.contains("Space Graph"), "{metric_view}");
    let (x, y) = find_text_position_in_area(&buffer, area, "214")
        .expect("registered Processes value should render");
    assert_eq!(buffer[(x, y)].fg, app.theme().active_series);
    assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));

    app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
        .unwrap();
    assert_eq!(
        app::clipboard::last_copied_text().as_deref(),
        Some("Processes\t214")
    );

    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(!app.show_cpu_core_dialog);

    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(app.selected_cpu_metric(), None);
    assert!(app.cpu_per_core_selected());
    let per_core_view = render_app_to_text(&app, screen.width, screen.height);
    assert!(per_core_view.contains("Enter Open"), "{per_core_view}");
    assert!(!per_core_view.contains("Space Graph"), "{per_core_view}");
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(app.show_cpu_core_dialog);
}

#[test]
fn cpu_inactive_graph_colors_the_value_without_bold_or_an_ordinal() {
    let mut app = make_test_app(3, 10);
    app.snapshot.cpu_total_usage_percent = Some(73);
    let ids = (0..9)
        .map(|index| add_test_graph(&mut app, index))
        .collect::<Vec<_>>();
    assert!(app.add_or_reveal_graph_source(
        GraphSlot::system(SystemMetric::CpuAverage),
        FocusedPanel::Cpu,
    ));
    assert!(app.set_active_graph(ids[0]));

    let screen = Rect::new(0, 0, 180, 45);
    let area = ui::cpu_panel_area_for_screen(screen, &app);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let (x, y) = find_text_position_in_area(&buffer, area, "73%")
        .expect("registered CPU value should render");
    let value = &buffer[(x, y)];

    assert_eq!(value.fg, app.theme().active_series);
    assert!(!value.modifier.contains(Modifier::BOLD));
    assert!(find_text_position_in_area(&buffer, area, "10 Usage").is_none());
}

#[test]
fn clicking_cpu_panel_moves_focus_to_cpu() {
    let mut app = make_test_app(3, 10);
    let screen = Rect::new(0, 0, 120, 45);
    let area = ui::cpu_panel_area_for_screen(screen, &app);

    app.on_mouse(left_click(area.x + 1, area.y + 1), screen);

    assert_eq!(app.focused_panel, FocusedPanel::Cpu);
    assert_eq!(app.status, "CPU row: CPU Usage");
}

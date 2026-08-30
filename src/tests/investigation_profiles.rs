use super::support::{
    assign_private_graph, find_text_position, make_test_app,
    record_tracked_process_history_samples, render_app_to_buffer, render_app_to_text,
    track_process_name, unique_config_path, unique_recording_path,
};
use crate::{
    app::{
        DetailsMetric, FocusedPanel, GraphDisplayMode, GraphSlot, GraphSlotLayout,
        InvestigationProfilesView, ProcessViewMode,
    },
    config::{
        InvestigationGraphConfig, InvestigationStartup, InvestigationStateConfig,
        SavedInvestigationProfile,
    },
    model::{
        GpuAdapterId, GpuAdapterSample, MetricColumn, ProcessIdentity, SortColumn, SortDirection,
        SystemMetric,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

fn profile(name: &str, graphs: Vec<InvestigationGraphConfig>) -> SavedInvestigationProfile {
    SavedInvestigationProfile {
        name: name.to_string(),
        investigation: InvestigationStateConfig {
            tracked_names: vec!["proc-0".to_string()],
            process_columns: vec!["CPU%".to_string(), "PrivBytes".to_string()],
            sort_by: "PrivBytes".to_string(),
            graphs,
            graph_columns: 2,
            graph_time_span_seconds: 300,
            samples: false,
            delta: false,
            y_axis_zero_min: false,
            recording_interval_seconds: 5,
            ..InvestigationStateConfig::default()
        },
    }
}

fn process_graph(name: &str, path: Option<&str>) -> InvestigationGraphConfig {
    InvestigationGraphConfig {
        kind: "process".to_string(),
        metric: "private_bytes".to_string(),
        display_mode: "raw".to_string(),
        process_name: Some(name.to_string()),
        executable_path: path.map(str::to_string),
        gpu_adapter_name: None,
    }
}

fn system_graph(metric: &str) -> InvestigationGraphConfig {
    InvestigationGraphConfig {
        kind: "system".to_string(),
        metric: metric.to_string(),
        display_mode: "raw".to_string(),
        process_name: None,
        executable_path: None,
        gpu_adapter_name: None,
    }
}

#[test]
fn save_as_captures_reusable_intent_without_runtime_identity() {
    let mut app = make_test_app(1, 10);
    app.snapshot.processes[0].name = "api.exe".to_string();
    app.snapshot.processes[0].executable_path = Some(r"C:\apps\api.exe".to_string());
    app.selected_process_identity = Some(ProcessIdentity::from_row(&app.snapshot.processes[0]));
    app.watch_list = vec!["api.exe".to_string()];
    app.watch_enabled = true;
    app.process_view_mode = ProcessViewMode::Tree;
    app.process_columns = vec![MetricColumn::CpuPercent, MetricColumn::PrivateBytes];
    app.sort.column = SortColumn::Metric(MetricColumn::CpuPercent);
    app.sort.direction = SortDirection::Asc;
    assign_private_graph(&mut app);
    app.graph_entries[0].display_mode = GraphDisplayMode::MovingAverage5;
    app.graph_slot_layout = GraphSlotLayout::TwoColumns;
    app.graph_time_span_seconds = 300;
    app.show_samples_panel = false;
    app.show_sample_delta = false;
    app.graph_y_axis_zero_min = false;
    app.recording_interval_index = 2;

    app.open_investigation_profiles();
    app.on_key(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT))
        .unwrap();
    for ch in "API check".chars() {
        app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
            .unwrap();
    }
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.runtime.saved_investigation_profiles.len(), 1);
    assert_eq!(
        app.active_investigation_profile.as_deref(),
        Some("API check")
    );
    let saved = &app.runtime.saved_investigation_profiles[0];
    assert_eq!(saved.tracked_names, ["api.exe"]);
    assert!(saved.tracked_only);
    assert_eq!(saved.process_view, "Tree");
    assert_eq!(saved.sort_by, "CPU%");
    assert_eq!(saved.sort_order, "asc");
    assert_eq!(saved.graph_columns, 2);
    assert_eq!(saved.graph_time_span_seconds, 300);
    assert!(!saved.samples);
    assert!(!saved.delta);
    assert!(!saved.y_axis_zero_min);
    assert_eq!(saved.recording_interval_seconds, 5);
    assert_eq!(saved.graphs.len(), 1);
    assert_eq!(saved.graphs[0].display_mode, "ma5");
    assert_eq!(saved.graphs[0].process_name.as_deref(), Some("api.exe"));
    assert_eq!(
        saved.graphs[0].executable_path.as_deref(),
        Some(r"C:\apps\api.exe")
    );
    let rendered = toml::to_string(saved).unwrap();
    assert!(!rendered.contains("pid"), "{rendered}");
    assert!(!rendered.contains("start_time"), "{rendered}");
    assert!(!rendered.contains("graph_id"), "{rendered}");
}

#[test]
fn save_rename_delete_and_duplicate_names_are_explicit() {
    let mut app = make_test_app(1, 10);
    app.runtime.saved_investigation_profiles = vec![profile("First", Vec::new())];
    app.active_investigation_profile = Some("First".to_string());
    app.open_investigation_profiles();

    app.graph_time_span_seconds = 600;
    app.on_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE))
        .unwrap();
    assert_eq!(
        app.runtime.saved_investigation_profiles[0].graph_time_span_seconds,
        600
    );

    app.on_key(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT))
        .unwrap();
    for ch in "first".chars() {
        app.push_investigation_profile_name_char(ch);
    }
    app.commit_investigation_profile_name_input();
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::NameInput {
            error: Some(error),
            ..
        }) if error.contains("already exists")
    ));

    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))
        .unwrap();
    for _ in 0.."First".len() {
        app.pop_investigation_profile_name_char();
    }
    for ch in "Renamed".chars() {
        app.push_investigation_profile_name_char(ch);
    }
    app.commit_investigation_profile_name_input();
    assert_eq!(app.runtime.saved_investigation_profiles[0].name, "Renamed");
    assert_eq!(app.active_investigation_profile.as_deref(), Some("Renamed"));

    app.on_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE))
        .unwrap();
    assert!(app.runtime.saved_investigation_profiles.is_empty());
    assert_eq!(app.active_investigation_profile, None);
}

#[test]
fn loading_resolves_a_restarted_process_to_its_current_identity() {
    let mut app = make_test_app(1, 10);
    app.snapshot.processes[0].name = "api.exe".to_string();
    app.snapshot.processes[0].pid = 902;
    app.snapshot.processes[0].start_time = Some(99_002);
    app.snapshot.processes[0].executable_path = Some(r"C:\apps\api.exe".to_string());
    let mut saved = profile(
        "API",
        vec![process_graph("API.EXE", Some(r"c:\APPS\API.EXE"))],
    );
    saved.tracked_names = vec!["api.exe".to_string()];
    saved.tracked_only = true;
    saved.process_view = "Tree".to_string();
    saved.sort_by = "CPU%".to_string();
    saved.sort_order = "Asc".to_string();
    saved.graphs[0].display_mode = "ma5".to_string();
    app.runtime.saved_investigation_profiles = vec![saved];
    app.ab_comparison = Some(crate::app::AbComparison { a: None, b: None });
    app.graph_time_offset_seconds = 120;
    app.graph_show_all_samples = true;

    app.open_investigation_profiles();
    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert!(app.investigation_profiles_dialog.is_none());
    assert_eq!(app.active_investigation_profile.as_deref(), Some("API"));
    assert_eq!(app.graph_entries.len(), 1);
    let GraphSlot::Process { identity, metric } = &app.graph_entries[0].source else {
        panic!("expected process Graph");
    };
    assert_eq!(identity.pid, 902);
    assert_eq!(identity.start_time, Some(99_002));
    assert_eq!(*metric, DetailsMetric::Private);
    assert_eq!(
        app.graph_entries[0].display_mode,
        GraphDisplayMode::MovingAverage5
    );
    assert!(app.watch_enabled);
    assert_eq!(app.process_view_mode, ProcessViewMode::Tree);
    assert_eq!(
        app.sort.column,
        SortColumn::Metric(MetricColumn::CpuPercent)
    );
    assert_eq!(app.sort.direction, SortDirection::Asc);
    assert_eq!(
        app.process_columns,
        [MetricColumn::CpuPercent, MetricColumn::PrivateBytes]
    );
    assert_eq!(app.graph_slot_layout, GraphSlotLayout::TwoColumns);
    assert_eq!(app.graph_time_span_seconds, 300);
    assert!(!app.show_samples_panel);
    assert!(!app.show_sample_delta);
    assert!(!app.graph_y_axis_zero_min);
    assert_eq!(app.selected_recording_interval_seconds(), 5);
    assert_eq!(app.ab_comparison, None);
    assert_eq!(app.graph_time_offset_seconds, 0);
    assert!(!app.graph_show_all_samples);
    assert!(!app.active_investigation_profile_dirty());
}

#[test]
fn tracked_only_remains_enabled_when_a_profile_has_no_tracked_names() {
    let mut app = make_test_app(1, 10);
    let mut saved = profile("Empty tracked-only", Vec::new());
    saved.tracked_names.clear();
    saved.tracked_only = true;
    app.runtime.saved_investigation_profiles = vec![saved];

    app.open_investigation_profiles();
    app.load_selected_investigation_profile();

    assert!(app.watch_list.is_empty());
    assert!(app.watch_enabled);
    assert_eq!(app.visible_process_count(), 0);
}

#[test]
fn startup_graph_restore_uses_new_ids_modes_and_reports_unresolved_templates() {
    let mut app = make_test_app(1, 10);
    let previous_next_id = app.next_graph_id;
    let mut restored = system_graph("cpu_average");
    restored.display_mode = "ma5".to_string();

    app.restore_initial_investigation_graphs(vec![restored, process_graph("missing.exe", None)]);

    assert_eq!(app.graph_entries.len(), 1);
    assert_eq!(app.graph_entries[0].id.0, previous_next_id);
    assert_eq!(
        app.graph_entries[0].display_mode,
        GraphDisplayMode::MovingAverage5
    );
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::LoadReport { unresolved, .. })
            if unresolved.len() == 1 && unresolved[0].contains("Process target unavailable")
    ));
}

#[test]
fn profile_dialog_changes_the_unified_startup_mode() {
    let mut app = make_test_app(1, 10);
    app.open_investigation_profiles();

    app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
        .unwrap();

    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::Startup {
            selected: InvestigationStartup::ChooseProfile
        })
    ));
    assert_eq!(
        app.runtime.investigation_startup,
        InvestigationStartup::ResumeLast
    );
    let rendered = render_app_to_text(&app, 100, 45);
    assert!(rendered.contains("STARTUP BEHAVIOR"), "{rendered}");
    assert!(rendered.contains("> Choose Profile"), "{rendered}");
    assert!(rendered.contains("Ask which Profile to load"), "{rendered}");

    app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(
        app.runtime.investigation_startup,
        InvestigationStartup::ChooseProfile
    );
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::Browse)
    ));
}

#[test]
fn profile_dialog_separates_current_selected_and_startup_without_fixed_list_gap() {
    let mut app = make_test_app(1, 10);
    app.runtime.saved_investigation_profiles = vec![profile("monitor-winproc-tui", Vec::new())];
    app.open_investigation_profiles();

    let buffer = render_app_to_buffer(&app, 76, 35);
    let rendered = super::support::buffer_to_text(&buffer);
    assert!(rendered.contains("CURRENT INVESTIGATION"), "{rendered}");
    assert!(rendered.contains("Not saved as a Profile"), "{rendered}");
    assert!(rendered.contains("SAVED PROFILES"), "{rendered}");
    assert!(
        rendered.contains("SELECTED PROFILE · monitor-winproc-tui"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Startup behavior: Resume last"),
        "{rendered}"
    );
    assert!(rendered.contains("u Startup"), "{rendered}");
    assert!(rendered.contains("F2 Rename"), "{rendered}");
    assert!(rendered.contains("Delete Delete"), "{rendered}");
    assert!(!rendered.contains("Current: Unsaved"), "{rendered}");
    assert!(!rendered.contains("(*)"), "{rendered}");

    let (_, profile_row) =
        find_text_position(&buffer, "> monitor-winproc-tui").expect("profile row should render");
    let (_, selected_heading_row) = find_text_position(&buffer, "SELECTED PROFILE")
        .expect("selected profile heading should render");
    assert_eq!(
        selected_heading_row,
        profile_row + 2,
        "selected profile details should follow a one-row profile list without a fixed empty gap"
    );
}

#[test]
fn profile_dialog_startup_mode_has_mouse_parity() {
    let mut app = make_test_app(1, 10);
    app.open_investigation_profiles();
    let screen = Rect::new(0, 0, 100, 45);
    let mut link = None;
    for y in 0..screen.height {
        for x in 0..screen.width {
            if crate::ui::investigation_profile_startup_link_at_for_screen(
                screen,
                x,
                y,
                app.investigation_profiles_entry_count(),
            ) {
                link = Some((x, y));
                break;
            }
        }
        if link.is_some() {
            break;
        }
    }
    let (column, row) = link.expect("Startup behavior should have a hit region");
    app.on_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        screen,
    );
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::Startup { .. })
    ));

    let mut hit = None;
    for y in 0..screen.height {
        for x in 0..screen.width {
            if crate::ui::investigation_profile_startup_at_for_screen(screen, x, y)
                == Some(InvestigationStartup::ChooseProfile)
            {
                hit = Some((x, y));
                break;
            }
        }
        if hit.is_some() {
            break;
        }
    }
    let (column, row) = hit.expect("Choose Profile should have a hit region");

    app.on_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        screen,
    );

    assert_eq!(
        app.runtime.investigation_startup,
        InvestigationStartup::ChooseProfile
    );
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::Browse)
    ));
}

#[test]
fn loading_a_process_template_without_a_path_constraint_stays_clean() {
    let mut app = make_test_app(1, 10);
    app.snapshot.processes[0].name = "api.exe".to_string();
    app.snapshot.processes[0].executable_path = Some(r"C:\apps\api.exe".to_string());
    app.runtime.saved_investigation_profiles =
        vec![profile("API", vec![process_graph("api.exe", None)])];

    app.open_investigation_profiles();
    app.load_selected_investigation_profile();

    assert_eq!(app.graph_entries.len(), 1);
    assert!(!app.active_investigation_profile_dirty());
}

#[test]
fn ambiguous_process_and_missing_gpu_templates_are_reported_without_guessing() {
    let mut app = make_test_app(2, 10);
    for process in &mut app.snapshot.processes {
        process.name = "worker.exe".to_string();
    }
    app.snapshot.gpu_adapters = vec![GpuAdapterSample {
        id: GpuAdapterId { high: 1, low: 2 },
        name: Some("Current GPU".to_string()),
        ..GpuAdapterSample::default()
    }];
    app.runtime.saved_investigation_profiles = vec![profile(
        "Unresolved",
        vec![
            process_graph("worker.exe", None),
            process_graph("missing.exe", None),
            InvestigationGraphConfig {
                kind: "gpu".to_string(),
                metric: "gpu_utilization".to_string(),
                display_mode: "raw".to_string(),
                process_name: None,
                executable_path: None,
                gpu_adapter_name: Some("Removed GPU".to_string()),
            },
        ],
    )];

    app.open_investigation_profiles();
    app.load_selected_investigation_profile();

    assert!(app.graph_entries.is_empty());
    let Some(InvestigationProfilesView::LoadReport { unresolved, .. }) =
        app.investigation_profiles_view()
    else {
        panic!("expected unresolved report");
    };
    assert_eq!(unresolved.len(), 3);
    assert!(unresolved[0].contains("Process target ambiguous"));
    assert!(unresolved[1].contains("Process target unavailable"));
    assert!(unresolved[2].contains("GPU adapter unavailable"));
    let rendered = render_app_to_text(&app, 100, 45);
    assert!(
        rendered.contains("Unresolved templates were not guessed or redirected."),
        "{rendered}"
    );
    assert!(rendered.contains("Process target ambiguous"), "{rendered}");
}

#[test]
fn loading_skips_duplicate_sources_and_stops_at_sixteen_graphs() {
    let mut app = make_test_app(1, 10);
    let metrics = [
        "cpu_average",
        "physical_memory",
        "modified_memory",
        "standby_memory",
        "free_zeroed_memory",
        "committed_memory",
        "paged_pool",
        "nonpaged_pool",
        "pages_input_per_sec",
        "pages_output_per_sec",
        "thread_count",
        "process_count",
        "network_received",
        "network_sent",
        "disk_read",
        "disk_write",
        "disk_queue_length",
    ];
    let mut graphs = metrics.into_iter().map(system_graph).collect::<Vec<_>>();
    graphs.insert(1, system_graph("cpu_average"));
    app.runtime.saved_investigation_profiles = vec![profile("Many", graphs)];

    app.open_investigation_profiles();
    app.load_selected_investigation_profile();

    assert_eq!(app.graph_entries.len(), 16);
    let Some(InvestigationProfilesView::LoadReport { unresolved, .. }) =
        app.investigation_profiles_view()
    else {
        panic!("expected load report");
    };
    assert!(
        unresolved
            .iter()
            .any(|line| line.contains("Duplicate source"))
    );
    assert!(
        unresolved
            .iter()
            .any(|line| line.contains("Graph limit reached"))
    );
}

#[test]
fn profile_load_uses_retained_history_confirmation() {
    let mut app = make_test_app(1, 10);
    track_process_name(&mut app, "old.exe");
    record_tracked_process_history_samples(&mut app, "old.exe", 180);
    let mut saved = profile("Next", Vec::new());
    saved.tracked_names = vec!["new.exe".to_string()];
    app.runtime.saved_investigation_profiles = vec![saved];

    app.open_investigation_profiles();
    app.load_selected_investigation_profile();

    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::ConfirmLoad { pending })
            if pending.tracking_switch.discarded_sample_count > 0
    ));
    assert_eq!(app.watch_list, ["old.exe"]);

    app.confirm_investigation_profile_action();

    assert_eq!(app.watch_list, ["new.exe"]);
}

#[test]
fn profile_load_persists_the_active_profile_and_current_investigation_immediately() {
    let mut app = make_test_app(1, 10);
    let mut saved = profile("Persisted", Vec::new());
    saved.tracked_names = vec!["persisted.exe".to_string()];
    app.runtime.saved_investigation_profiles = vec![saved];
    let path = unique_config_path("profile-load");
    app.runtime.config_path = Some(path.clone());

    app.open_investigation_profiles();
    app.load_selected_investigation_profile();

    let loaded = crate::config::load_config(&path).unwrap();
    let runtime = crate::config::build_runtime_config(loaded).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        runtime.active_investigation_profile.as_deref(),
        Some("Persisted")
    );
    assert_eq!(runtime.process_filters, ["persisted.exe"]);
}

#[test]
fn ctrl_t_opens_profiles_and_save_load_are_rejected_outside_live() {
    let mut app = make_test_app(1, 10);
    app.on_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL))
        .unwrap();
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::Browse)
    ));
    let rendered = render_app_to_text(&app, 100, 45);
    assert!(rendered.contains("INVESTIGATION PROFILES"), "{rendered}");
    assert!(rendered.contains("CURRENT INVESTIGATION"), "{rendered}");
    assert!(rendered.contains("Not saved as a Profile"), "{rendered}");
    assert!(rendered.contains("S Save New"), "{rendered}");
    assert!(rendered.contains("u Startup"), "{rendered}");

    app.close_investigation_profiles();
    app.log_view_path = Some("loaded.log".into());
    app.open_investigation_profiles();
    app.runtime.saved_investigation_profiles = vec![profile("Blocked", Vec::new())];
    app.load_selected_investigation_profile();
    assert_eq!(app.active_investigation_profile, None);
    assert!(app.status.contains("Log view"));
    app.begin_save_investigation_profile_as();
    assert!(matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::Browse)
    ));
    assert!(app.status.contains("Log view"));

    app.log_view_path = None;
    let path = unique_recording_path("profiles-rejected");
    let _ = std::fs::remove_file(&path);
    track_process_name(&mut app, "proc-0");
    app.recording_path_draft = path.display().to_string();
    app.recording_path_cursor = app.recording_path_draft.len();
    app.show_recording_path_dialog = true;
    app.confirm_recording_path().unwrap();
    app.load_selected_investigation_profile();
    assert_eq!(app.active_investigation_profile, None);
    assert!(app.status.contains("Recording"));
    app.begin_save_investigation_profile_as();
    assert!(app.status.contains("Recording"));
    app.stop_recording().unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn one_gpu_name_match_resolves_to_the_current_adapter_id() {
    let mut app = make_test_app(1, 10);
    app.snapshot.gpu_adapters = vec![GpuAdapterSample {
        id: GpuAdapterId { high: 7, low: 9 },
        name: Some("Test GPU".to_string()),
        ..GpuAdapterSample::default()
    }];
    app.runtime.saved_investigation_profiles = vec![profile(
        "GPU",
        vec![InvestigationGraphConfig {
            kind: "gpu".to_string(),
            metric: "gpu_dedicated".to_string(),
            display_mode: "raw".to_string(),
            process_name: None,
            executable_path: None,
            gpu_adapter_name: Some("test gpu".to_string()),
        }],
    )];

    app.open_investigation_profiles();
    app.load_selected_investigation_profile();

    assert_eq!(
        app.graph_entries[0].source,
        GraphSlot::gpu(
            GpuAdapterId { high: 7, low: 9 },
            "test gpu",
            SystemMetric::GpuDedicated
        )
    );
    assert_eq!(app.focused_panel, FocusedPanel::Processes);
}

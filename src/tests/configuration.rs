use super::support::{make_test_app, unique_config_path};
use crate::app::{
    GraphSlotLayout, ProcessPanelHeight, SAMPLE_STALE_AFTER_SECONDS, SampleFreshness,
};
use crate::cli::Cli;
use crate::config;
use crate::config::{AppConfig, build_runtime_config, load_config, write_app_config};
use crate::model::{ColumnPreset, MetricColumn, SortColumn, SortDirection, SortSpec};
use crate::with_terminal_session;
use clap::Parser;
use std::time::Duration;

#[test]
fn terminal_session_does_not_restore_between_startup_and_main() {
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let setup_events = std::rc::Rc::clone(&events);
    let operation_events = std::rc::Rc::clone(&events);
    let restore_events = std::rc::Rc::clone(&events);

    with_terminal_session(
        || {
            setup_events.borrow_mut().push("setup");
            Ok(())
        },
        |_| {
            let mut events = operation_events.borrow_mut();
            events.push("startup choice");
            events.push("initial sample");
            events.push("main loop");
            Ok(())
        },
        |_| {
            restore_events.borrow_mut().push("restore");
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        events.borrow().as_slice(),
        [
            "setup",
            "startup choice",
            "initial sample",
            "main loop",
            "restore"
        ]
    );
}

#[test]
fn terminal_session_restores_after_startup_failure() {
    let restored = std::cell::Cell::new(false);

    let result = with_terminal_session(
        || Ok(()),
        |_| Err::<(), _>(anyhow::anyhow!("startup failed")),
        |_| {
            restored.set(true);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(restored.get());
}

#[test]
fn build_runtime_config_uses_config_process_filters() {
    let mut config = AppConfig::default();
    config.tracked.push(config::TrackedConfig {
        name: "app.exe".to_string(),
    });

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.process_filters, vec!["app.exe"]);
}

#[test]
fn build_runtime_config_restores_process_table_settings() {
    let mut config = AppConfig::default();
    config.process_table.preset = "Custom".to_string();
    config.process_table.columns = vec!["CPU %".to_string(), "Private".to_string()];
    config.process_table.sort_by = "CPU %".to_string();
    config.process_table.sort_order = "asc".to_string();
    config.process_table.tracked_only = true;

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.column_preset, ColumnPreset::Custom);
    assert_eq!(
        runtime.process_columns,
        vec![MetricColumn::CpuPercent, MetricColumn::PrivateBytes]
    );
    assert_eq!(
        runtime.sort,
        SortSpec {
            column: SortColumn::Metric(MetricColumn::CpuPercent),
            direction: SortDirection::Asc,
        }
    );
    assert!(runtime.initial_tracked_only);
}

#[test]
fn process_panel_height_defaults_to_auto_when_the_setting_is_missing() {
    let config: AppConfig = toml::from_str(
        r#"
[process_table]
preset = "Default"
"#,
    )
    .unwrap();

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(
        runtime.initial_process_panel_height,
        ProcessPanelHeight::Auto
    );
}

#[test]
fn process_panel_height_loads_manual_rows_and_rejects_invalid_values() {
    let manual: AppConfig = toml::from_str(
        r#"
[process_table]
body_rows = 14
"#,
    )
    .unwrap();
    assert_eq!(
        build_runtime_config(manual)
            .unwrap()
            .initial_process_panel_height,
        ProcessPanelHeight::Manual(14)
    );

    for value in ["0", "-1", "65536", "\"invalid\""] {
        let config: AppConfig =
            toml::from_str(&format!("[process_table]\nbody_rows = {value}\n")).unwrap();
        assert_eq!(
            build_runtime_config(config)
                .unwrap()
                .initial_process_panel_height,
            ProcessPanelHeight::Auto,
            "value={value}"
        );
    }
}

#[test]
fn process_panel_height_round_trips_manual_and_auto_settings() {
    let path = unique_config_path("process-panel-height");
    let mut app = make_test_app(20, 10);
    app.process_panel_height = ProcessPanelHeight::Manual(14);

    write_app_config(&path, &app).unwrap();
    let manual_rendered = std::fs::read_to_string(&path).unwrap();
    let manual_runtime = build_runtime_config(load_config(&path).unwrap()).unwrap();
    assert!(
        manual_rendered.contains("body_rows = 14"),
        "{manual_rendered}"
    );
    assert_eq!(
        manual_runtime.initial_process_panel_height,
        ProcessPanelHeight::Manual(14)
    );

    app.process_panel_height = ProcessPanelHeight::Auto;
    write_app_config(&path, &app).unwrap();
    let auto_rendered = std::fs::read_to_string(&path).unwrap();
    let auto_runtime = build_runtime_config(load_config(&path).unwrap()).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(
        auto_rendered.contains("body_rows = \"auto\""),
        "{auto_rendered}"
    );
    assert_eq!(
        auto_runtime.initial_process_panel_height,
        ProcessPanelHeight::Auto
    );
}

#[test]
fn default_runtime_config_selects_all_process_columns() {
    let runtime = build_runtime_config(AppConfig::default()).unwrap();

    assert_eq!(runtime.process_columns, MetricColumn::ALL);
    assert_eq!(
        runtime.process_column_widths.resolved(SortColumn::Pid),
        SortColumn::Pid.default_width()
    );
}

#[test]
fn removed_gc_rate_columns_are_ignored_in_saved_config() {
    let mut config = AppConfig::default();
    config.process_table.preset = "Custom".to_string();
    config.process_table.columns = vec![
        ".NET GC0/s".to_string(),
        "CPU%".to_string(),
        ".NET GC2/s".to_string(),
    ];
    config.process_table.sort_by = ".NET GC1/s".to_string();

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.process_columns, vec![MetricColumn::CpuPercent]);
    assert_eq!(
        runtime.sort.column,
        SortColumn::Metric(MetricColumn::WorksetPrivateBytes)
    );
}

#[test]
fn build_runtime_config_restores_and_clamps_column_width_overrides() {
    let mut config = AppConfig::default();
    config.process_table.column_widths.extend([
        ("PID".to_string(), -10),
        ("Process".to_string(), 999),
        ("Private".to_string(), 14),
        ("Full Path".to_string(), 60),
        ("WS Shrbl".to_string(), 40),
        ("Unknown".to_string(), 50),
    ]);

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.process_column_widths.resolved(SortColumn::Pid), 5);
    assert_eq!(
        runtime
            .process_column_widths
            .resolved(SortColumn::ProcessName),
        120
    );
    assert_eq!(
        runtime
            .process_column_widths
            .resolved(SortColumn::Metric(MetricColumn::PrivateBytes)),
        14
    );
    assert_eq!(
        runtime
            .process_column_widths
            .resolved(SortColumn::Metric(MetricColumn::FullPath)),
        60
    );
    assert_eq!(
        runtime
            .process_column_widths
            .resolved(SortColumn::Metric(MetricColumn::WorksetShareableBytes)),
        40
    );
}

#[test]
fn tracked_entries_do_not_enable_tracked_only_without_saved_state() {
    let mut config = AppConfig::default();
    config.tracked.push(config::TrackedConfig {
        name: "app.exe".to_string(),
    });

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.process_filters, vec!["app.exe"]);
    assert!(!runtime.initial_tracked_only);
}

#[test]
fn build_runtime_config_falls_back_when_custom_columns_are_empty() {
    let mut config = AppConfig::default();
    config.process_table.preset = "Custom".to_string();
    config.process_table.columns.clear();

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.column_preset, ColumnPreset::Custom);
    assert_eq!(
        runtime.process_columns,
        ColumnPreset::Default.columns().to_vec()
    );
}

#[test]
fn cli_rejects_removed_runtime_options() {
    let removed_args: &[&[&str]] = &[
        &["winproc-tui", "-c", "C:/work/winproc-tui.toml"],
        &["winproc-tui", "--config", "C:/work/winproc-tui.toml"],
        &["winproc-tui", "-p", "app.exe"],
        &["winproc-tui", "--process", "app.exe"],
        &["winproc-tui", "--preset", "io"],
        &["winproc-tui", "--no-mouse"],
        &["winproc-tui", "--interval", "5"],
        &["winproc-tui", "--ws-share"],
        &["winproc-tui", "--no-ws-share"],
        &["winproc-tui", "--no-gpu-metrics"],
        &["winproc-tui", "--no-gui-resources"],
        &["winproc-tui", "config"],
        &["winproc-tui", "config", "init"],
        &["winproc-tui", "config", "path"],
    ];

    for args in removed_args {
        assert!(Cli::try_parse_from(*args).is_err());
    }
}

#[test]
fn build_runtime_config_restores_recording_last_dir() {
    let mut config = AppConfig::default();
    let last_dir = std::path::PathBuf::from("C:/reports/winproc-tui");
    config.recording.last_dir = Some(last_dir.clone());

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.recording_last_dir, Some(last_dir));
}

#[test]
fn runtime_config_uses_no_recording_dir_by_default() {
    let config = AppConfig::default();

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.recording_last_dir, None);
}

#[test]
fn build_runtime_config_restores_graph_view_settings() {
    let mut config = AppConfig::default();
    config.graphs.columns = 2;
    config.graphs.samples = false;
    config.graphs.delta = false;

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(
        runtime.initial_graph_slot_layout,
        GraphSlotLayout::TwoColumns
    );
    assert!(!runtime.initial_show_samples_panel);
    assert!(!runtime.initial_show_sample_delta);
}

#[test]
fn invalid_graph_column_count_falls_back_to_auto() {
    let mut config = AppConfig::default();
    config.graphs.columns = 4;

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.initial_graph_slot_layout, GraphSlotLayout::Auto);
}

#[test]
fn build_runtime_config_restores_three_column_graph_layout() {
    let mut config = AppConfig::default();
    config.graphs.columns = 3;

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(
        runtime.initial_graph_slot_layout,
        GraphSlotLayout::ThreeColumns
    );
}

#[test]
fn cli_rejects_export_dir_option() {
    let error = Cli::try_parse_from(["winproc-tui", "--export-dir", "C:/logs"]).unwrap_err();

    assert!(error.to_string().contains("unexpected argument"));
}

#[test]
fn sampling_interval_is_fixed_to_one_second() {
    let app = make_test_app(1, 10);

    assert_eq!(app.tick_interval(), Duration::from_secs(1));
}

#[test]
fn app_config_accepts_and_omits_legacy_interval_seconds() {
    let config: AppConfig = toml::from_str(
        r#"
[general]
interval_seconds = 30
mouse = false
theme = "Light"
"#,
    )
    .unwrap();

    assert!(!config.general.mouse);
    assert_eq!(config.general.theme, "Light");

    let rendered = toml::to_string(&config).unwrap();
    assert!(!rendered.contains("interval_seconds"), "{rendered}");
}

#[test]
fn sample_freshness_turns_stale_at_defined_threshold() {
    let mut app = make_test_app(1, 10);
    let captured_at = app.snapshot.captured_at;

    assert_eq!(
        app.sample_freshness_at(
            captured_at + chrono::Duration::seconds(SAMPLE_STALE_AFTER_SECONDS as i64 - 1)
        ),
        Some(SampleFreshness::Fresh)
    );
    assert_eq!(
        app.sample_freshness_at(
            captured_at + chrono::Duration::seconds(SAMPLE_STALE_AFTER_SECONDS as i64)
        ),
        Some(SampleFreshness::Stale {
            age_seconds: SAMPLE_STALE_AFTER_SECONDS
        })
    );

    app.log_view_path = Some(std::path::PathBuf::from("session.log"));
    assert_eq!(app.sample_freshness_at(captured_at), None);
}

#[test]
fn app_config_accepts_legacy_process_entries_as_watch_list() {
    let config: AppConfig = toml::from_str(
        r#"
[[process]]
name = "legacy.exe"
"#,
    )
    .unwrap();

    assert_eq!(config.tracked[0].name, "legacy.exe");
}

#[test]
fn app_config_accepts_legacy_watch_entries_as_tracked_list() {
    let config: AppConfig = toml::from_str(
        r#"
[[watch]]
name = "legacy-watch.exe"
"#,
    )
    .unwrap();

    assert_eq!(config.tracked[0].name, "legacy-watch.exe");
}

#[test]
fn app_config_saves_tracked_entries() {
    let mut config = AppConfig::default();
    config.tracked.push(config::TrackedConfig {
        name: "app.exe".to_string(),
    });

    let rendered = toml::to_string(&config).unwrap();

    assert!(rendered.contains("[[tracked]]"));
    assert!(!rendered.contains("[[watch]]"));
}

#[test]
fn app_config_accepts_named_tracked_lists_and_startup_mode() {
    let config: AppConfig = toml::from_str(
        r#"
[tracking]
startup = "choose_list"
active_list = "API"

[[tracked_lists]]
name = "API"
processes = ["api.exe", "worker.exe"]
"#,
    )
    .unwrap();

    assert_eq!(
        config.tracking.startup,
        config::TrackedListStartup::ChooseList
    );
    assert_eq!(config.tracking.active_list.as_deref(), Some("API"));
    assert_eq!(config.tracked_lists[0].name, "API");
    assert_eq!(
        config.tracked_lists[0].processes,
        vec!["api.exe", "worker.exe"]
    );
}

#[test]
fn start_empty_runtime_ignores_last_working_tracked_list() {
    let mut config = AppConfig::default();
    config.tracking.startup = config::TrackedListStartup::StartEmpty;
    config.tracking.active_list = Some("API".to_string());
    config.tracked.push(config::TrackedConfig {
        name: "api.exe".to_string(),
    });

    let runtime = build_runtime_config(config).unwrap();

    assert!(runtime.process_filters.is_empty());
    assert_eq!(runtime.active_tracked_list, None);
}

#[test]
fn runtime_normalizes_named_tracked_lists_case_insensitively() {
    let mut config = AppConfig::default();
    config.tracked_lists = vec![
        config::SavedTrackedList {
            name: " API ".to_string(),
            processes: vec![
                "api.exe".to_string(),
                "API.EXE".to_string(),
                " worker.exe ".to_string(),
            ],
        },
        config::SavedTrackedList {
            name: "api".to_string(),
            processes: vec!["duplicate.exe".to_string()],
        },
        config::SavedTrackedList {
            name: " empty (DEFAULT) ".to_string(),
            processes: Vec::new(),
        },
    ];

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.saved_tracked_lists.len(), 1);
    assert_eq!(runtime.saved_tracked_lists[0].name, "API");
    assert_eq!(
        runtime.saved_tracked_lists[0].processes,
        vec!["api.exe", "worker.exe"]
    );
}

#[test]
fn runtime_does_not_restore_builtin_empty_as_a_named_list() {
    let mut config = AppConfig::default();
    config.tracking.active_list = Some("Empty (default)".to_string());
    config.tracked_lists = vec![config::SavedTrackedList {
        name: "Empty (default)".to_string(),
        processes: Vec::new(),
    }];

    let runtime = build_runtime_config(config).unwrap();

    assert_eq!(runtime.active_tracked_list, None);
    assert!(runtime.saved_tracked_lists.is_empty());
}

#[test]
fn app_config_saves_named_tracked_lists_and_startup_mode() {
    let mut app = make_test_app(1, 10);
    app.runtime.tracked_list_startup = config::TrackedListStartup::ChooseList;
    app.runtime.active_tracked_list = Some("API".to_string());
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "API".to_string(),
        processes: vec!["api.exe".to_string()],
    }];
    let path = unique_config_path("named-tracked-lists");

    write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(rendered.contains("startup = \"choose_list\""), "{rendered}");
    assert!(rendered.contains("active_list = \"API\""), "{rendered}");
    assert!(rendered.contains("[[tracked_lists]]"), "{rendered}");
    assert!(rendered.contains("processes = [\"api.exe\"]"), "{rendered}");
}

#[test]
fn app_config_never_writes_builtin_empty_as_a_named_list() {
    let mut app = make_test_app(1, 10);
    app.runtime.active_tracked_list = Some("Empty (default)".to_string());
    app.runtime.saved_tracked_lists = vec![config::SavedTrackedList {
        name: "Empty (default)".to_string(),
        processes: Vec::new(),
    }];
    let path = unique_config_path("builtin-empty-tracked-list");

    config::write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(!rendered.contains("Empty (default)"), "{rendered}");
    assert!(!rendered.contains("[[tracked_lists]]"), "{rendered}");
}

#[test]
fn app_config_saves_tracked_only_state() {
    let mut app = make_test_app(3, 10);
    app.watch_enabled = true;
    let path = unique_config_path("tracked-only");

    write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(rendered.contains("tracked_only = true"), "{rendered}");
}

#[test]
fn app_config_saves_selected_color_scheme() {
    let mut app = make_test_app(3, 10);
    app.theme_index = 3;
    let path = unique_config_path("color-scheme");

    write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let loaded = load_config(&path).unwrap();
    let runtime = build_runtime_config(loaded).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(rendered.contains("theme = \"Cyan\""), "{rendered}");
    assert_eq!(runtime.initial_theme, "Cyan");
}

#[test]
fn app_config_saves_graph_layout_and_explicit_samples_preference() {
    let mut app = make_test_app(3, 10);
    app.graph_slot_layout = GraphSlotLayout::TwoColumns;
    app.show_samples_panel = false;
    app.show_sample_delta = false;
    let path = unique_config_path("graph-layout");

    write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(rendered.contains("[graphs]"), "{rendered}");
    assert!(rendered.contains("columns = 2"), "{rendered}");
    assert!(rendered.contains("samples = false"), "{rendered}");
    assert!(rendered.contains("delta = false"), "{rendered}");
}

#[test]
fn app_config_saves_auto_graph_layout() {
    let app = make_test_app(3, 10);
    let path = unique_config_path("graph-layout-auto");

    write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(rendered.contains("[graphs]"), "{rendered}");
    assert!(rendered.contains("columns = 0"), "{rendered}");
}

#[test]
fn app_config_round_trips_only_non_default_column_widths() {
    let mut app = make_test_app(3, 10);
    app.process_column_widths.set(SortColumn::Pid, 8);
    app.process_column_widths
        .set(SortColumn::Metric(MetricColumn::PrivateBytes), 14);
    app.process_column_widths.set(
        SortColumn::ProcessName,
        SortColumn::ProcessName.default_width(),
    );
    let path = unique_config_path("column-widths");

    write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let loaded = load_config(&path).unwrap();
    let runtime = build_runtime_config(loaded).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(
        rendered.contains("[process_table.column_widths]"),
        "{rendered}"
    );
    assert!(rendered.contains("PID = 8"), "{rendered}");
    assert!(rendered.contains("PrivBytes = 14"), "{rendered}");
    assert!(!rendered.contains("Process = 18"), "{rendered}");
    assert_eq!(runtime.process_column_widths.resolved(SortColumn::Pid), 8);
    assert_eq!(
        runtime
            .process_column_widths
            .resolved(SortColumn::Metric(MetricColumn::PrivateBytes)),
        14
    );
}

#[test]
fn app_config_does_not_save_filter_state() {
    let mut app = make_test_app(3, 10);
    app.filter_text = "proc".to_string();
    let path = unique_config_path("filter");

    write_app_config(&path, &app).unwrap();
    let rendered = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert!(!rendered.contains("[filter]"), "{rendered}");
    assert!(!rendered.contains("initial ="), "{rendered}");
    assert!(!rendered.contains("initial = \"proc\""), "{rendered}");
}

#[test]
fn app_config_saves_recording_last_dir() {
    let mut config = AppConfig::default();
    config.recording.last_dir = Some(std::path::PathBuf::from("C:/logs"));

    let rendered = toml::to_string(&config).unwrap();

    assert!(rendered.contains("[recording]"));
    assert!(rendered.contains("last_dir"));
}

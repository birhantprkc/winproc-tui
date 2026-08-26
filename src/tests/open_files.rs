use super::support::{
    buffer_to_text, find_text_position, make_test_app, make_test_app_with_workers,
    render_app_to_buffer, render_app_to_text, show_process_info_files_tab, test_open_files_report,
};
use crate::app;
use crate::app::FocusedPanel;
use crate::samplers::SamplingWorker;
use crate::samplers::open_files::{
    OpenFileEntry, OpenFilesError, OpenFilesReport, OpenFilesRequest, OpenFilesResult,
    OpenFilesWorker,
};
use crate::samplers::process_info::ProcessInfoWorker;
use crate::ui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::{Position, Rect};
use std::sync::mpsc::TryRecvError;

#[test]
fn f_requests_open_files_for_selected_process() {
    let (sampling_worker, _, _) = SamplingWorker::test_pair();
    let (process_info_worker, _, _) = ProcessInfoWorker::test_pair();
    let (open_files_worker, request_rx, _) = OpenFilesWorker::test_pair();
    let mut app = make_test_app_with_workers(
        2,
        10,
        sampling_worker,
        process_info_worker,
        open_files_worker,
    );
    app.process_info_tab = app::ProcessInfoTab::Environment;

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE))
        .unwrap();

    assert!(app.show_process_info_dialog);
    assert_eq!(app.process_info_tab, app::ProcessInfoTab::Files);
    assert_eq!(app.open_files_in_flight.as_ref().unwrap().name, "proc-0");
    match request_rx.try_recv().unwrap() {
        OpenFilesRequest::Collect {
            identity, process, ..
        } => {
            assert_eq!(identity.name, "proc-0");
            assert_eq!(process.name, "proc-0");
        }
        OpenFilesRequest::Stop => panic!("unexpected stop request"),
    }
}

#[test]
fn f_does_not_open_files_outside_processes_focus() {
    let (sampling_worker, _, _) = SamplingWorker::test_pair();
    let (process_info_worker, _, _) = ProcessInfoWorker::test_pair();
    let (open_files_worker, request_rx, _) = OpenFilesWorker::test_pair();
    let mut app = make_test_app_with_workers(
        2,
        10,
        sampling_worker,
        process_info_worker,
        open_files_worker,
    );
    app.focused_panel = FocusedPanel::System;

    app.on_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE))
        .unwrap();

    assert!(!app.show_process_info_dialog);
    assert!(request_rx.try_recv().is_err());
}

#[test]
fn ctrl_u_refreshes_open_files_for_selected_process() {
    let (sampling_worker, _, _) = SamplingWorker::test_pair();
    let (process_info_worker, _, _) = ProcessInfoWorker::test_pair();
    let (open_files_worker, request_rx, _) = OpenFilesWorker::test_pair();
    let mut app = make_test_app_with_workers(
        2,
        10,
        sampling_worker,
        process_info_worker,
        open_files_worker,
    );
    app.open_selected_process_info_dialog().unwrap();
    app.process_info_tab = app::ProcessInfoTab::Files;
    app.process_info_focus = app::ProcessInfoFocus::Content;
    let identity = app.process_info_target.as_ref().unwrap().identity.clone();
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 1,
        file_handles: 1,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![OpenFileEntry {
            path: r"C:\tmp\a.log".to_string(),
            handle_count: 1,
        }],
        error: None,
    });
    app.open_files_result_identity = Some(identity);

    app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL))
        .unwrap();

    assert!(app.open_files_result.is_some());
    assert_eq!(app.open_files_in_flight.as_ref().unwrap().name, "proc-0");
    match request_rx.try_recv().unwrap() {
        OpenFilesRequest::Collect {
            identity, process, ..
        } => {
            assert_eq!(identity.name, "proc-0");
            assert_eq!(process.name, "proc-0");
        }
        OpenFilesRequest::Stop => panic!("unexpected stop request"),
    }
}

#[test]
fn open_files_result_updates_modal_state() {
    let (sampling_worker, _, _) = SamplingWorker::test_pair();
    let (process_info_worker, _, _) = ProcessInfoWorker::test_pair();
    let (open_files_worker, request_rx, result_tx) = OpenFilesWorker::test_pair();
    let mut app = make_test_app_with_workers(
        1,
        10,
        sampling_worker,
        process_info_worker,
        open_files_worker,
    );
    app.open_selected_process_files().unwrap();
    let (generation, identity) = match request_rx.try_recv().unwrap() {
        OpenFilesRequest::Collect {
            generation,
            identity,
            ..
        } => (generation, identity),
        OpenFilesRequest::Stop => panic!("unexpected stop request"),
    };

    result_tx
        .send(OpenFilesResult {
            generation,
            identity: identity.clone(),
            report: OpenFilesReport {
                pid: 0,
                process_name: "proc-0".to_string(),
                total_handles: 3,
                file_handles: 2,
                inaccessible_handles: 1,
                unnamed_file_handles: 0,
                entries: vec![OpenFileEntry {
                    path: r"C:\tmp\a.log".to_string(),
                    handle_count: 2,
                }],
                error: None,
            },
        })
        .unwrap();

    assert!(app.poll_open_files_results().unwrap());
    assert!(app.open_files_in_flight.is_none());
    assert_eq!(app.open_files_result.as_ref().unwrap().entries.len(), 1);
    assert!(app.status.contains("Loaded 1 open file paths"));
}

#[test]
fn open_files_clipboard_is_raw_paths_without_header() {
    let mut app = make_test_app(1, 10);
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 2,
        file_handles: 2,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![
            OpenFileEntry {
                path: r"C:\tmp\a.log".to_string(),
                handle_count: 1,
            },
            OpenFileEntry {
                path: r"C:\tmp\b.log".to_string(),
                handle_count: 2,
            },
        ],
        error: None,
    });

    app.copy_open_files_to_clipboard().unwrap();

    assert_eq!(
        crate::app::clipboard::last_copied_text().unwrap(),
        "C:\\tmp\\a.log\nC:\\tmp\\b.log\t2"
    );
}

#[test]
fn open_files_clipboard_filter_matches_full_paths() {
    let mut app = make_test_app(1, 10);
    app.open_files_filter = "exports".to_string();
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 3,
        file_handles: 3,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![
            OpenFileEntry {
                path: r"C:\tmp\a.wav".to_string(),
                handle_count: 1,
            },
            OpenFileEntry {
                path: r"C:\exports\b.MXF".to_string(),
                handle_count: 2,
            },
            OpenFileEntry {
                path: r"C:\media\c.mp4".to_string(),
                handle_count: 1,
            },
        ],
        error: None,
    });

    app.copy_open_files_to_clipboard().unwrap();

    assert_eq!(
        crate::app::clipboard::last_copied_text().unwrap(),
        "C:\\exports\\b.MXF\t2"
    );
}

#[test]
fn open_files_filter_cursor_moves_and_inserts_at_cursor() {
    let mut app = make_test_app(1, 10);
    show_process_info_files_tab(&mut app);
    app.open_files_filter = ".mp4".to_string();
    app.open_files_filter_cursor = app.open_files_filter.len();

    app.on_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.open_files_filter, ".mpx4");
    assert_eq!(app.open_files_filter_cursor, ".mpx".len());

    app.on_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.open_files_filter, ".mp4");
    assert_eq!(app.open_files_filter_cursor, app.open_files_filter.len());
}

#[test]
fn open_files_filter_delete_removes_character_at_cursor() {
    let mut app = make_test_app(1, 10);
    show_process_info_files_tab(&mut app);
    app.open_files_filter = ".mxpf".to_string();
    app.open_files_filter_cursor = ".mx".len();

    app.on_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))
        .unwrap();

    assert_eq!(app.open_files_filter, ".mxf");
    assert_eq!(app.open_files_filter_cursor, ".mx".len());
}

#[test]
fn open_files_filter_shows_colon_and_terminal_cursor() {
    let mut app = make_test_app(1, 10);
    show_process_info_files_tab(&mut app);
    app.open_files_filter = ".mp4".to_string();
    app.open_files_filter_cursor = ".m".len();
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 1,
        file_handles: 1,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![OpenFileEntry {
            path: r"C:\tmp\a.mp4".to_string(),
            handle_count: 1,
        }],
        error: None,
    });
    let screen = Rect::new(0, 0, 160, 45);
    let content = ui::process_info_content_area_for_screen(screen);
    let expected_cursor = Position::new(content.x + 10, content.y + 1);

    let backend = TestBackend::new(screen.width, screen.height);
    let mut terminal = Terminal::new(backend).expect("test terminal should be created");
    terminal
        .draw(|frame| ui::draw(frame, &app))
        .expect("test render should succeed");
    terminal
        .backend_mut()
        .assert_cursor_position(expected_cursor);
    let rendered = buffer_to_text(terminal.backend().buffer());

    assert!(rendered.contains("Filter: .mp4"), "{rendered}");
}

#[test]
fn open_files_modal_size_stays_fixed_while_filtering() {
    let mut app = make_test_app(1, 10);
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 3,
        file_handles: 3,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![
            OpenFileEntry {
                path: r"C:\tmp\a.log".to_string(),
                handle_count: 1,
            },
            OpenFileEntry {
                path: r"C:\tmp\b.log".to_string(),
                handle_count: 1,
            },
            OpenFileEntry {
                path: r"C:\tmp\c.log".to_string(),
                handle_count: 1,
            },
        ],
        error: None,
    });
    let screen = Rect::new(0, 0, 160, 45);
    show_process_info_files_tab(&mut app);
    let before = ui::process_info_page_size_for_screen(screen);

    app.open_files_filter = "b.log".to_string();
    let after = ui::process_info_page_size_for_screen(screen);

    assert_eq!(before, after);
}

#[test]
fn open_files_modal_renders_table_columns() {
    let mut app = make_test_app(1, 10);
    show_process_info_files_tab(&mut app);
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 1,
        file_handles: 1,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![OpenFileEntry {
            path: r"C:\tmp\a.log".to_string(),
            handle_count: 1,
        }],
        error: None,
    });

    let rendered = render_app_to_text(&app, 160, 45);

    assert!(rendered.contains("Count File"), "{rendered}");
    assert!(rendered.contains("a.log"), "{rendered}");
    assert!(rendered.contains(r"C:\tmp"), "{rendered}");
}

#[test]
fn open_files_filter_matches_directory_and_shows_filtered_total() {
    let mut app = make_test_app(1, 10);
    show_process_info_files_tab(&mut app);
    app.open_files_filter = "fonts".to_string();
    app.open_files_filter_cursor = app.open_files_filter.len();
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 2,
        file_handles: 2,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![
            OpenFileEntry {
                path: r"C:\Windows\Fonts\a.ttf".to_string(),
                handle_count: 1,
            },
            OpenFileEntry {
                path: r"C:\tmp\b.log".to_string(),
                handle_count: 1,
            },
        ],
        error: None,
    });

    let rendered = render_app_to_text(&app, 120, 30);

    assert!(rendered.contains("shown 1/2"), "{rendered}");
    assert!(rendered.contains("a.ttf"), "{rendered}");
    assert!(!rendered.contains("b.log"), "{rendered}");
}

#[test]
fn open_files_table_column_names_are_underlined() {
    let mut app = make_test_app(1, 10);
    show_process_info_files_tab(&mut app);
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 1,
        file_handles: 1,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: vec![OpenFileEntry {
            path: r"C:\tmp\a.log".to_string(),
            handle_count: 1,
        }],
        error: None,
    });

    let buffer = render_app_to_buffer(&app, 160, 45);
    let (x, y) = find_text_position(&buffer, "Count").expect("header should render");
    let cell = &buffer[(x, y)];

    assert!(cell.modifier.contains(ratatui::style::Modifier::UNDERLINED));
    assert!(cell.modifier.contains(ratatui::style::Modifier::BOLD));
}

#[test]
fn open_files_scroll_offset_changes_rendered_rows() {
    let mut app = make_test_app(1, 10);
    show_process_info_files_tab(&mut app);
    app.open_files_result = Some(OpenFilesReport {
        pid: 0,
        process_name: "proc-0".to_string(),
        total_handles: 30,
        file_handles: 30,
        inaccessible_handles: 0,
        unnamed_file_handles: 0,
        entries: (0..30)
            .map(|index| OpenFileEntry {
                path: format!(r"C:\tmp\file-{index:02}.log"),
                handle_count: 1,
            })
            .collect(),
        error: None,
    });
    let screen = Rect::new(0, 0, 160, 45);
    app.set_process_info_page_size(ui::process_info_page_size_for_screen(screen));
    app.scroll_process_info_end();

    let rendered = render_app_to_text(&app, screen.width, screen.height);

    assert!(!rendered.contains("file-00.log"), "{rendered}");
    assert!(rendered.contains("file-29.log"), "{rendered}");
}

#[test]
fn files_tab_in_log_view_does_not_request_live_collection() {
    let (sampling_worker, _, _) = SamplingWorker::test_pair();
    let (process_info_worker, process_request_rx, _) = ProcessInfoWorker::test_pair();
    let (open_files_worker, open_files_request_rx, _) = OpenFilesWorker::test_pair();
    let mut app = make_test_app_with_workers(
        1,
        10,
        sampling_worker,
        process_info_worker,
        open_files_worker,
    );
    app.log_view_path = Some(std::path::PathBuf::from("recording.log"));

    app.open_selected_process_info_dialog().unwrap();
    app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL))
        .unwrap();
    app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL))
        .unwrap();

    assert!(matches!(
        process_request_rx.try_recv(),
        Err(TryRecvError::Empty)
    ));
    assert!(matches!(
        open_files_request_rx.try_recv(),
        Err(TryRecvError::Empty)
    ));
    let rendered = render_app_to_text(&app, 120, 40);
    assert!(rendered.contains("Not recorded in Log view."), "{rendered}");
}

#[test]
fn files_tab_does_not_query_after_the_fixed_target_exits() {
    let (sampling_worker, _, _) = SamplingWorker::test_pair();
    let (process_info_worker, _, _) = ProcessInfoWorker::test_pair();
    let (open_files_worker, open_files_request_rx, _) = OpenFilesWorker::test_pair();
    let mut app = make_test_app_with_workers(
        1,
        10,
        sampling_worker,
        process_info_worker,
        open_files_worker,
    );

    app.open_selected_process_info_dialog().unwrap();
    app.snapshot.processes.clear();
    app.activate_process_info_tab(app::ProcessInfoTab::Files)
        .unwrap();

    assert!(matches!(
        open_files_request_rx.try_recv(),
        Err(TryRecvError::Empty)
    ));
    assert_eq!(
        app.open_files_result
            .as_ref()
            .and_then(|report| report.error.as_ref()),
        Some(&OpenFilesError::ProcessExited)
    );
    assert_eq!(app.status, "Process has exited");
}

#[test]
fn stale_open_files_result_cannot_replace_reopened_dialog_request() {
    let (sampling_worker, _, _) = SamplingWorker::test_pair();
    let (process_info_worker, _, _) = ProcessInfoWorker::test_pair();
    let (open_files_worker, request_rx, result_tx) = OpenFilesWorker::test_pair();
    let mut app = make_test_app_with_workers(
        1,
        10,
        sampling_worker,
        process_info_worker,
        open_files_worker,
    );

    app.open_selected_process_files().unwrap();
    let (old_generation, identity) = match request_rx.try_recv().unwrap() {
        OpenFilesRequest::Collect {
            generation,
            identity,
            ..
        } => (generation, identity),
        OpenFilesRequest::Stop => panic!("unexpected stop request"),
    };
    app.close_process_info_dialog();
    app.open_selected_process_files().unwrap();
    let new_generation = match request_rx.try_recv().unwrap() {
        OpenFilesRequest::Collect { generation, .. } => generation,
        OpenFilesRequest::Stop => panic!("unexpected stop request"),
    };

    result_tx
        .send(OpenFilesResult {
            generation: old_generation,
            identity: identity.clone(),
            report: test_open_files_report(&identity.name, identity.pid, "old.log"),
        })
        .unwrap();
    assert!(!app.poll_open_files_results().unwrap());
    assert_eq!(app.open_files_in_flight_generation, Some(new_generation));
    assert!(app.open_files_result.is_none());

    result_tx
        .send(OpenFilesResult {
            generation: new_generation,
            identity: identity.clone(),
            report: test_open_files_report(&identity.name, identity.pid, "new.log"),
        })
        .unwrap();
    assert!(app.poll_open_files_results().unwrap());
    assert!(
        app.open_files_result.as_ref().unwrap().entries[0]
            .path
            .ends_with("new.log")
    );
}

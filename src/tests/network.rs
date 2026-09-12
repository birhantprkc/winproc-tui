use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, TryRecvError},
};

use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::Modifier};

use super::support::{
    find_text_position, left_click, make_test_app, render_app_to_buffer, render_app_to_text,
    track_process_name, unique_recording_path,
};
use crate::{
    App,
    app::{AppActivity, ProcessInfoFocus, ProcessInfoTab, sync_layout_state},
    model::network::{
        NetworkEndpoint, NetworkEndpointKey, NetworkOwner, NetworkProtocol, NetworkReport,
    },
    samplers::network::{
        NetworkContext, NetworkPayload, NetworkRequest, NetworkResult, NetworkWorker,
    },
    ui,
};

fn setup() -> (App, Receiver<NetworkRequest>, Sender<NetworkResult>) {
    let mut app = make_test_app(2, 10);
    let (worker, requests, results) = NetworkWorker::test_pair();
    app.network_worker = worker;
    (app, requests, results)
}

fn capture(requests: &Receiver<NetworkRequest>) -> NetworkContext {
    let NetworkRequest::Collect(context) = requests.try_recv().unwrap() else {
        panic!("expected collection")
    };
    context
}

fn report(app: &App) -> NetworkReport {
    let owner = NetworkOwner {
        identity: crate::model::ProcessIdentity::from_row(&app.snapshot.processes[0]),
        executable_path: app.snapshot.processes[0].executable_path.clone(),
        creation_time: 123456,
    };
    let endpoint = |protocol, local: &str, remote: Option<&str>, state| NetworkEndpoint {
        key: NetworkEndpointKey {
            protocol,
            local: local.parse().unwrap(),
            remote: remote.map(|value| value.parse().unwrap()),
            pid: owner.identity.pid,
        },
        tcp_state: state,
        owner: Some(owner.clone()),
    };
    NetworkReport {
        started_at: Local::now(),
        captured_at: Local::now(),
        failures: vec![],
        successful_tables: 4,
        endpoints: vec![
            endpoint(NetworkProtocol::Tcp4, "127.0.0.1:8080", None, Some(2)),
            endpoint(
                NetworkProtocol::Tcp6,
                "[fe80::1234%7]:54321",
                Some("[2001:db8:1234:5678:abcd:ef12:3456:789a]:443"),
                Some(5),
            ),
            endpoint(NetworkProtocol::Udp4, "0.0.0.0:5353", None, None),
        ],
    }
}

fn deliver(
    app: &mut App,
    results: &Sender<NetworkResult>,
    context: NetworkContext,
    report: NetworkReport,
) {
    results
        .send(NetworkResult {
            context,
            payload: NetworkPayload::Report(Ok(report)),
        })
        .unwrap();
    assert!(app.poll_network_results());
}

fn press(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap();
}

#[test]
fn network_browser_rejects_old_generations_and_deduplicates_refresh() {
    let (mut app, requests, results) = setup();
    app.open_network_browser();
    let old = capture(&requests);
    app.refresh_network(true);
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    press(&mut app, KeyCode::Esc);
    app.open_network_browser();
    let current = capture(&requests);
    assert_ne!(old.generation, current.generation);
    results
        .send(NetworkResult {
            context: old,
            payload: NetworkPayload::Report(Ok(report(&app))),
        })
        .unwrap();
    assert!(!app.poll_network_results());
    assert_eq!(app.network_browser.pending.as_ref(), Some(&current));
    let snapshot = report(&app);
    deliver(&mut app, &results, current, snapshot);
    assert_eq!(app.network_browser.entries().len(), 2);
    press(&mut app, KeyCode::Char('a'));
    assert_eq!(app.network_browser.entries().len(), 3);
    press(&mut app, KeyCode::Char('/'));
    for ch in "2001:db8".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.network_browser.entries().len(), 1);
    assert_eq!(
        app.network_browser.selected_entry().unwrap().key.protocol,
        NetworkProtocol::Tcp6
    );
    press(&mut app, KeyCode::Char('r'));
    let refresh = capture(&requests);
    results
        .send(NetworkResult {
            context: refresh,
            payload: NetworkPayload::Report(Err("TCP table unavailable".into())),
        })
        .unwrap();
    assert!(app.poll_network_results());
    assert_eq!(app.network_browser.entries().len(), 1);
    assert_eq!(app.network_browser.filter, "2001:db8");
    assert!(
        app.network_browser
            .notice
            .as_ref()
            .unwrap()
            .contains("unavailable")
    );
}

#[test]
fn network_owner_navigation_is_verified_and_returns_to_retained_results() {
    let (mut app, requests, results) = setup();
    let snapshot = report(&app);
    let expected_identity = snapshot.endpoints[0].owner.as_ref().unwrap().identity();
    app.open_network_browser();
    deliver(&mut app, &results, capture(&requests), snapshot);
    press(&mut app, KeyCode::Enter);
    let NetworkRequest::Verify(context, owner) = requests.try_recv().unwrap() else {
        panic!("owner must be reverified")
    };
    assert!(!app.show_process_info_dialog);
    // Selection and a new process with the same PID must not retarget navigation.
    app.snapshot.processes[0].start_time = Some(9999);
    app.process_table_state.select(Some(1));
    results
        .send(NetworkResult {
            context,
            payload: NetworkPayload::Owner(Ok(owner)),
        })
        .unwrap();
    assert!(app.poll_network_results());
    assert_eq!(
        app.process_info_target.as_ref().unwrap().identity,
        expected_identity
    );
    assert_eq!(app.process_info_tab, ProcessInfoTab::Network);
    assert!(app.process_network.all);
    let process_capture = capture(&requests);
    assert_eq!(process_capture.target.as_ref(), Some(&expected_identity));
    press(&mut app, KeyCode::Esc);
    assert!(!app.show_process_info_dialog);
    assert!(app.network_browser.visible);
    assert_eq!(app.network_browser.entries().len(), 2);
    results
        .send(NetworkResult {
            context: process_capture,
            payload: NetworkPayload::Report(Ok(report(&app))),
        })
        .unwrap();
    assert!(!app.poll_network_results());
    assert!(app.process_network.report.is_none());
    press(&mut app, KeyCode::Esc);
    assert!(!app.network_browser.visible);
}

#[test]
fn network_unresolved_and_exited_owners_cannot_navigate() {
    let (mut app, requests, results) = setup();
    let mut snapshot = report(&app);
    snapshot.endpoints[0].owner = None;
    app.open_network_browser();
    deliver(&mut app, &results, capture(&requests), snapshot);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    assert!(!app.show_process_info_dialog);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    let NetworkRequest::Verify(context, _) = requests.try_recv().unwrap() else {
        panic!("verification")
    };
    results
        .send(NetworkResult {
            context,
            payload: NetworkPayload::Owner(Err("Process exited".into())),
        })
        .unwrap();
    assert!(app.poll_network_results());
    assert!(!app.show_process_info_dialog);
    assert_eq!(
        app.network_browser.notice.as_deref(),
        Some("Process exited")
    );
}

#[test]
fn network_process_tab_is_lazy_fixed_and_preserves_cached_results_across_tabs() {
    let (mut app, requests, results) = setup();
    app.open_selected_process_info_dialog().unwrap();
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    app.activate_process_info_tab(ProcessInfoTab::Network)
        .unwrap();
    let context = capture(&requests);
    let snapshot = report(&app);
    app.process_table_state.select(Some(1));
    deliver(&mut app, &results, context.clone(), snapshot);
    assert_eq!(app.process_network.target, context.target);
    app.activate_process_info_tab(ProcessInfoTab::Metrics)
        .unwrap();
    app.activate_process_info_tab(ProcessInfoTab::Network)
        .unwrap();
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    assert_eq!(app.process_network.entries().len(), 3);
    app.close_process_info_dialog();
    app.open_selected_process_info_dialog().unwrap();
    let reopened = capture(&requests);
    assert_ne!(context.generation, reopened.generation);
    let mut wrong_request = reopened.clone();
    wrong_request.id = context.id;
    results
        .send(NetworkResult {
            context: wrong_request,
            payload: NetworkPayload::Report(Ok(report(&app))),
        })
        .unwrap();
    assert!(!app.poll_network_results());
    assert_eq!(app.process_network.pending.as_ref(), Some(&reopened));
}

#[test]
fn network_process_filter_accepts_direct_text_without_intercepting_navigation() {
    let (mut app, requests, results) = setup();
    app.open_selected_process_info_dialog().unwrap();
    app.activate_process_info_tab(ProcessInfoTab::Network)
        .unwrap();
    let mut snapshot = report(&app);
    for entry in &mut snapshot.endpoints {
        entry.owner.as_mut().unwrap().identity.name = "rare-player.exe".into();
    }
    deliver(&mut app, &results, capture(&requests), snapshot.clone());
    press(&mut app, KeyCode::Char('x'));
    assert!(app.process_network.filter.is_empty());
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.process_info_focus, ProcessInfoFocus::Content);
    for ch in "rare/ 映像".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    assert_eq!(app.process_network.filter, "rare/ 映像");
    assert!(!app.process_network.editing);
    assert!(app.process_network.all);
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.process_network.filter, "rare/ 像");
    press(&mut app, KeyCode::Delete);
    press(&mut app, KeyCode::Backspace);
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.process_network.filter, "rare");
    assert_eq!(app.process_network.entries().len(), 3);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.process_network.selected, 1);
    press(&mut app, KeyCode::End);
    assert_eq!(app.process_network.selected, 2);
    press(&mut app, KeyCode::Home);
    assert_eq!(app.process_network.selected, 0);
    assert_eq!(app.process_network.cursor, 4);
    app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL))
        .unwrap();
    deliver(&mut app, &results, capture(&requests), snapshot);
    assert_eq!(app.process_network.filter, "rare");
    app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT))
        .unwrap();
    assert!(!app.process_network.all);
    assert_eq!(app.process_network.entries().len(), 2);
    assert_eq!(app.process_network.filter, "rare");
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    press(&mut app, KeyCode::Enter);
    assert!(app.process_network.detail);
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.process_network.filter, "rare");
    press(&mut app, KeyCode::Esc);
    assert!(!app.process_network.detail);
    assert!(app.show_process_info_dialog);
    press(&mut app, KeyCode::Esc);
    assert!(!app.show_process_info_dialog);
}

#[test]
fn network_process_filter_click_focuses_direct_input_and_shows_matching_shortcuts() {
    let (mut app, requests, results) = setup();
    app.open_selected_process_info_dialog().unwrap();
    app.activate_process_info_tab(ProcessInfoTab::Network)
        .unwrap();
    let snapshot = report(&app);
    deliver(&mut app, &results, capture(&requests), snapshot);
    let screen = Rect::new(0, 0, 100, 30);
    sync_layout_state(&mut app, screen);
    let layout = ui::network::content_layout(ui::process_info_content_area_for_screen(screen));
    app.on_mouse(left_click(layout.filter.x, layout.filter.y), screen);
    assert_eq!(app.process_info_focus, ProcessInfoFocus::Content);
    assert!(!app.process_network.editing);
    press(&mut app, KeyCode::Char('r'));
    assert_eq!(app.process_network.filter, "r");
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    assert_eq!(
        buffer[(layout.filter.x, layout.filter.y)].bg,
        app.theme().focus_surface
    );
    let text = render_app_to_text(&app, screen.width, screen.height);
    assert!(text.contains("Ctrl+U refresh"));
    assert!(text.contains("[Alt+A] All"));
    assert!(!text.contains("/ filter"));
    press(&mut app, KeyCode::Esc);
    assert!(!app.show_process_info_dialog);
}

#[test]
fn network_log_view_performs_no_collection_and_discards_live_results() {
    let (mut app, requests, results) = setup();
    app.open_network_browser();
    let pending = capture(&requests);
    app.network_browser.visible = false;
    app.log_view_path = Some(PathBuf::from("example.jsonl"));
    app.open_selected_process_info_dialog().unwrap();
    app.activate_process_info_tab(ProcessInfoTab::Network)
        .unwrap();
    app.process_info_focus = ProcessInfoFocus::Content;
    app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL))
        .unwrap();
    app.open_network_browser();
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    results
        .send(NetworkResult {
            context: pending,
            payload: NetworkPayload::Report(Ok(report(&app))),
        })
        .unwrap();
    assert!(!app.poll_network_results());
    assert!(render_app_to_text(&app, 100, 30).contains("Not recorded in Log view."));
    assert!(!app.network_browser.visible);
}

#[test]
fn network_recording_continues_without_serializing_endpoint_reports() {
    let (mut app, requests, results) = setup();
    let path = unique_recording_path("network-investigation");
    track_process_name(&mut app, "proc-0");
    app.recording_path_draft = path.to_string_lossy().into_owned();
    app.recording_path_cursor = app.recording_path_draft.len();
    app.show_recording_path_dialog = true;
    app.confirm_recording_path().unwrap();
    app.open_network_browser();
    let snapshot = report(&app);
    deliver(&mut app, &results, capture(&requests), snapshot);
    assert_eq!(app.activity(), AppActivity::Recording);
    app.stop_recording().unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(!saved.contains("127.0.0.1:8080"));
    assert!(!saved.contains("endpoints"));
    assert!(!saved.contains("creation_time"));
    assert!(saved.lines().count() >= 2);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn network_filter_unicode_details_copy_and_mouse_geometry_remain_consistent() {
    let (mut app, requests, results) = setup();
    let mut snapshot = report(&app);
    snapshot.endpoints[1].owner.as_mut().unwrap().identity.name = "映像-player.exe".into();
    app.open_network_browser();
    deliver(&mut app, &results, capture(&requests), snapshot);
    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Char('/'));
    for ch in "映像x".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.network_browser.filter, "映像");
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Delete);
    assert_eq!(app.network_browser.filter, "像");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.network_browser.entries().len(), 1);
    let full_copy = app.network_browser.selected_entry().unwrap().plain_text();
    assert!(full_copy.contains("[2001:db8:1234:5678:abcd:ef12:3456:789a]:443"));
    assert_eq!(full_copy.split('\t').count(), 6);
    for (width, height) in [(60, 18), (80, 24), (150, 40)] {
        let screen = Rect::new(0, 0, width, height);
        sync_layout_state(&mut app, screen);
        let layout = ui::network::content_layout(ui::network::browser_layout(screen).content);
        app.on_mouse(left_click(layout.rows.x, layout.rows.y), screen);
        assert_eq!(app.network_browser.selected, 0);
        let buffer = render_app_to_buffer(&app, width, height);
        let (x, y) = find_text_position(&buffer, "TCP6").expect("selected TCP6 row");
        assert!(buffer[(x, y)].modifier.contains(Modifier::BOLD));
        assert_eq!(buffer[(x, y)].bg, app.theme().focus_surface);
        assert_eq!(y, layout.rows.y);
        press(&mut app, KeyCode::Char(' '));
        let detail = ui::network::detail_lines(&app.network_browser, layout.rows.width).join("");
        assert!(detail.contains("[2001:db8:1234:5678:abcd:ef12:3456:789a]:443"));
        press(&mut app, KeyCode::Esc);
        assert!(app.network_browser.visible);
    }
}

#[test]
fn network_partial_and_empty_captures_remain_distinguishable() {
    let (mut app, requests, results) = setup();
    let mut snapshot = report(&app);
    snapshot.endpoints.clear();
    snapshot.successful_tables = 3;
    snapshot.failures.push("TCP6: Windows error 5".into());
    app.open_network_browser();
    deliver(&mut app, &results, capture(&requests), snapshot);
    let text = render_app_to_text(&app, 140, 35);
    assert!(text.contains("3/4 tables"));
    assert!(text.contains("Partial: TCP6: Windows error 5"));
    assert!(text.contains("partial capture"));
    assert!(text.contains("refresh"));
    assert!(!app.should_quit);
}

#[test]
fn network_scroll_and_selection_survive_refresh_and_resize() {
    let (mut app, requests, results) = setup();
    let mut snapshot = report(&app);
    let base = snapshot.endpoints[0].clone();
    snapshot.endpoints = (0..60)
        .map(|index| {
            let mut entry = base.clone();
            entry.key.local.set_port(10000 + index);
            entry
        })
        .collect();
    app.open_network_browser();
    deliver(&mut app, &results, capture(&requests), snapshot.clone());
    sync_layout_state(&mut app, Rect::new(0, 0, 80, 24));
    press(&mut app, KeyCode::End);
    assert_eq!(app.network_browser.selected, 59);
    let selected = app.network_browser.selected_entry().unwrap().key.clone();
    press(&mut app, KeyCode::Char('r'));
    snapshot.endpoints.remove(0);
    deliver(&mut app, &results, capture(&requests), snapshot);
    assert_eq!(app.network_browser.selected_entry().unwrap().key, selected);
    assert_eq!(app.network_browser.selected, 58);
    sync_layout_state(&mut app, Rect::new(0, 0, 60, 16));
    assert!(app.network_browser.selected >= app.network_browser.scroll.offset);
    assert!(
        app.network_browser.selected
            < app.network_browser.scroll.offset + app.network_browser.scroll.page_size
    );
    let screen = Rect::new(0, 0, 60, 16);
    let bar = ui::network::scrollbar_area(
        ui::network::browser_layout(screen).content,
        &app.network_browser,
    )
    .unwrap();
    app.on_mouse(left_click(bar.x, bar.y), screen);
    assert_eq!(app.network_browser.selected, 0);
    assert_eq!(app.network_browser.scroll.offset, 0);
}

#[test]
fn network_process_detail_scroll_survives_layout_sync_and_uses_own_hit_regions() {
    let (mut app, requests, results) = setup();
    app.open_selected_process_info_dialog().unwrap();
    app.activate_process_info_tab(ProcessInfoTab::Network)
        .unwrap();
    app.process_info_focus = ProcessInfoFocus::Content;
    let mut snapshot = report(&app);
    snapshot.failures = (0..20)
        .map(|index| format!("TCP6: capture failure {index}"))
        .collect();
    deliver(&mut app, &results, capture(&requests), snapshot);
    let screen = Rect::new(0, 0, 80, 24);
    sync_layout_state(&mut app, screen);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::PageDown);
    let offset = app.process_network.scroll.offset;
    assert!(offset > 0);
    sync_layout_state(&mut app, screen);
    assert_eq!(app.process_network.scroll.offset, offset);
    assert!(render_app_to_text(&app, 80, 24).contains("capture failure"));
    let area = ui::process_info_content_area_for_screen(screen);
    let bar = ui::network::scrollbar_area(area, &app.process_network).unwrap();
    app.on_mouse(left_click(bar.x, bar.y), screen);
    assert_eq!(app.process_network.scroll.offset, 0);
    press(&mut app, KeyCode::Esc);
    assert!(!app.process_network.detail);
    assert!(app.show_process_info_dialog);
    let layout = ui::network::content_layout(area);
    app.on_mouse(left_click(layout.rows.x, layout.rows.y + 1), screen);
    assert_eq!(app.process_network.selected, 1);
    let buffer = render_app_to_buffer(&app, 80, 24);
    let (x, y) = super::support::find_text_position_in_area(&buffer, layout.rows, "TCP6").unwrap();
    assert_eq!(y, layout.rows.y + 1);
    assert_eq!(buffer[(x, y)].bg, app.theme().focus_surface);
}

#[test]
fn network_menu_opens_browser_and_late_owner_result_cannot_reopen_it() {
    let (mut app, requests, results) = setup();
    press(&mut app, KeyCode::Esc);
    app.main_menu_selected = app
        .main_menu_rows()
        .iter()
        .position(|row| app.main_menu_row_label(*row) == "Investigate ▸")
        .unwrap();
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Enter);
    assert!(app.network_browser.visible);
    assert!(!app.is_main_menu_open());
    let snapshot = report(&app);
    deliver(&mut app, &results, capture(&requests), snapshot);
    press(&mut app, KeyCode::Enter);
    let NetworkRequest::Verify(context, owner) = requests.try_recv().unwrap() else {
        panic!("verification")
    };
    press(&mut app, KeyCode::Esc);
    results
        .send(NetworkResult {
            context,
            payload: NetworkPayload::Owner(Ok(owner)),
        })
        .unwrap();
    assert!(!app.poll_network_results());
    assert!(!app.show_process_info_dialog);
    app.log_view_path = Some(PathBuf::from("example.jsonl"));
    press(&mut app, KeyCode::Esc);
    assert!(
        !app.main_menu_rows()
            .iter()
            .any(|row| app.main_menu_row_label(*row).contains("Investigate"))
    );
}

#[test]
fn network_wide_browser_renders_full_scoped_ipv6_endpoints_without_cell_clipping() {
    let (mut app, requests, results) = setup();
    let mut snapshot = report(&app);
    let mut entry = snapshot.endpoints[1].clone();
    let local = "[2001:db8:abcd:1234:5678:9abc:def0:1234%4294967295]:65535";
    let remote = "[ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff%4294967295]:65535";
    entry.key.local = local.parse().unwrap();
    entry.key.remote = Some(remote.parse().unwrap());
    entry.key.pid = u32::MAX;
    entry.owner.as_mut().unwrap().identity.name = "backgroundTaskHost.exe".into();
    snapshot.endpoints = vec![entry];
    app.open_network_browser();
    app.network_browser.all = true;
    deliver(&mut app, &results, capture(&requests), snapshot);
    let screen = Rect::new(0, 0, 200, 35);
    sync_layout_state(&mut app, screen);
    let modal = ui::network::browser_layout(screen);
    assert_eq!(modal.area.width, 184);
    let layout = ui::network::content_layout(modal.content);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let row = (layout.rows.x..layout.rows.right())
        .map(|x| buffer[(x, layout.rows.y)].symbol())
        .collect::<String>();
    for value in [
        local,
        remote,
        "ESTABLISHED",
        "4294967295",
        "backgroundTaskHost.exe",
    ] {
        assert!(row.contains(value), "Missing {value}: {row}");
    }
    assert!(!row.contains('…'), "{row}");
    let (state_x, _) =
        super::support::find_text_position_in_area(&buffer, layout.header, "State").unwrap();
    let (value_x, _) =
        super::support::find_text_position_in_area(&buffer, layout.rows, "ESTABLISHED").unwrap();
    assert_eq!(value_x, state_x);
    app.on_mouse(left_click(layout.rows.x, layout.rows.y), screen);
    assert_eq!(app.network_browser.selected, 0);
}

#[test]
fn network_shared_table_uses_available_width_for_both_ipv6_addresses() {
    let (mut app, requests, results) = setup();
    let mut snapshot = report(&app);
    let mut entry = snapshot.endpoints[1].clone();
    let local = "[2001:db8:abcd:1111:2222:3333:4444:5555]:54321";
    let remote = "[2001:db8:1234:5678:9abc:def0:1111:2222]:443";
    entry.key.local = local.parse().unwrap();
    entry.key.remote = Some(remote.parse().unwrap());
    entry.tcp_state = Some(8);
    entry.owner.as_mut().unwrap().identity.name = "backgroundTaskHost.exe".into();
    snapshot.endpoints = vec![entry];
    app.open_selected_process_info_dialog().unwrap();
    app.activate_process_info_tab(ProcessInfoTab::Network)
        .unwrap();
    deliver(&mut app, &results, capture(&requests), snapshot);
    let screen = Rect::new(0, 0, 160, 35);
    sync_layout_state(&mut app, screen);
    let buffer = render_app_to_buffer(&app, screen.width, screen.height);
    let layout = ui::network::content_layout(ui::process_info_content_area_for_screen(screen));
    let row = (layout.rows.x..layout.rows.right())
        .map(|x| buffer[(x, layout.rows.y)].symbol())
        .collect::<String>();
    for value in [local, remote, "CLOSE_WAIT", "backgroundTaskHost.exe"] {
        assert!(row.contains(value), "Missing {value}: {row}");
    }
    assert!(!row.contains('…'), "{row}");
}

#[test]
fn network_tcp_state_labels_match_windows_netstat_in_details_filter_and_copy() {
    let app = make_test_app(1, 10);
    let mut entry = report(&app).endpoints[0].clone();
    let mut view = crate::app::network::NetworkView {
        all: true,
        report: Some(report(&app)),
        ..crate::app::network::NetworkView::default()
    };
    for (code, expected) in [
        (1, "CLOSED"),
        (2, "LISTENING"),
        (3, "SYN_SENT"),
        (4, "SYN_RECEIVED"),
        (5, "ESTABLISHED"),
        (6, "FIN_WAIT_1"),
        (7, "FIN_WAIT_2"),
        (8, "CLOSE_WAIT"),
        (9, "CLOSING"),
        (10, "LAST_ACK"),
        (11, "TIME_WAIT"),
        (12, "DELETE_TCB"),
        (0, "UNKNOWN"),
    ] {
        entry.tcp_state = Some(code);
        assert_eq!(entry.state_label(), expected);
        assert_eq!(entry.plain_text().split('\t').nth(3), Some(expected));
        assert!(entry.matches(&expected.to_ascii_lowercase()));
        view.report.as_mut().unwrap().endpoints = vec![entry.clone()];
        assert!(ui::network::detail_lines(&view, 80).contains(&format!("State: {expected}")));
    }
    entry.tcp_state = None;
    entry.key.protocol = NetworkProtocol::Udp4;
    assert_eq!(entry.state_label(), "");
    let copied = entry.plain_text();
    let fields = copied.split('\t').collect::<Vec<_>>();
    assert_eq!(fields.len(), 6);
    assert_eq!(fields[3], "");
    assert_eq!(fields[4], entry.key.pid.to_string());
    assert!(!entry.matches("Bound"));
    view.report.as_mut().unwrap().endpoints = vec![entry];
    assert!(ui::network::detail_lines(&view, 80).contains(&"State: ".to_string()));
}

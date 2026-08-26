mod app;
mod io;
mod process;
mod render;

pub(super) use app::{
    add_test_graph, assign_private_graph, make_test_app, make_test_app_with_worker,
    make_test_app_with_workers, populate_system_info, record_tracked_process_history_samples,
    selected_process_history_sample_count, test_graph_source, test_snapshot, track_process_name,
};
pub(super) use io::{
    AlwaysFailWriter, unique_config_path, unique_recording_dir, unique_recording_path,
};
pub(super) use process::{
    activate_process_environment_tab, activate_process_modules_tab, show_process_info_files_tab,
    test_open_files_report, test_process_environment_report, test_process_info,
    test_process_module_entry, test_process_modules_report,
};
pub(super) use render::{
    area_contains_foreground, assert_blank_row_above_text, assert_dialog_title_style,
    assert_modal_rect_focus_border, assert_title_style, buffer_to_text,
    find_styled_symbol_positions_in_area, find_symbol_position, find_text_position,
    find_text_position_in_area, left_click, mouse_move, render_app_to_buffer, render_app_to_text,
};

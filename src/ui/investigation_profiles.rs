use ratatui::{
    layout::{Alignment, Position, Rect},
    prelude::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::{
    App,
    app::{InvestigationProfilesView, ProfileNameInputPurpose},
    config::InvestigationStartup,
    ui::{
        Theme,
        footer::{shortcut_spans, warning_shortcut_spans},
        widgets::{
            block::{panel_block_focused, panel_title},
            confirm_dialog::{self, centered_dialog_rect},
        },
    },
};

const DIALOG_WIDTH: u16 = 88;
const CURRENT_LABEL_ROW: u16 = 0;
const CURRENT_ROW: u16 = 1;
const LIST_LABEL_ROW: u16 = 3;
const LIST_ROW: u16 = 4;
const MAX_LIST_HEIGHT: u16 = 6;
const NAME_DIALOG_WIDTH: u16 = 60;
const NAME_DIALOG_HEIGHT: u16 = 8;
const CONFIRM_DIALOG_WIDTH: u16 = 68;
const CONFIRM_DIALOG_HEIGHT: u16 = 8;
const REPORT_LIST_ROW: u16 = 4;
const REPORT_LIST_HEIGHT: u16 = 15;
const REPORT_DIALOG_HEIGHT: u16 = 25;
const REPORT_SHORTCUT_ROW: u16 = 22;
const STARTUP_DIALOG_WIDTH: u16 = 72;
const STARTUP_DIALOG_HEIGHT: u16 = 12;
const STARTUP_OPTION_ROW: u16 = 3;
const STARTUP_SHORTCUT_ROW: u16 = 9;

pub(crate) fn draw_investigation_profiles(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    app: &App,
    theme: Theme,
) {
    let Some(view) = app.investigation_profiles_view() else {
        return;
    };
    match view {
        InvestigationProfilesView::Startup { .. } => draw_startup(frame, area, app, theme),
        InvestigationProfilesView::LoadReport { .. } => draw_load_report(frame, area, app, theme),
        _ => {
            draw_browse(frame, area, app, theme);
            match view {
                InvestigationProfilesView::NameInput { .. } => {
                    draw_name_input(frame, area, app, theme)
                }
                InvestigationProfilesView::ConfirmDelete { .. } => {
                    draw_delete_confirm(frame, area, app, theme)
                }
                InvestigationProfilesView::ConfirmLoad { .. } => {
                    draw_load_confirm(frame, area, app, theme)
                }
                InvestigationProfilesView::Browse
                | InvestigationProfilesView::Startup { .. }
                | InvestigationProfilesView::LoadReport { .. } => {}
            }
        }
    }
}

fn draw_browse(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let layout = browse_layout(area, app.investigation_profiles_entry_count());
    let popup = layout.popup;
    let block = panel_block_focused(panel_title("INVESTIGATION PROFILES"), theme, true);
    let content = layout.content;
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    draw_section_label(
        frame,
        content,
        CURRENT_LABEL_ROW,
        "CURRENT INVESTIGATION",
        theme,
    );
    let current = app
        .active_investigation_profile
        .as_deref()
        .map(|name| {
            if app.active_investigation_profile_dirty() {
                format!("Profile: {name} (modified)")
            } else {
                format!("Profile: {name}")
            }
        })
        .unwrap_or_else(|| "Not saved as a Profile".to_string());
    frame.render_widget(
        Paragraph::new(current).style(Style::default().fg(theme.text)),
        row(content, CURRENT_ROW),
    );

    draw_section_label(frame, content, LIST_LABEL_ROW, "SAVED PROFILES", theme);

    let list_area = layout.list;
    let count = app.investigation_profiles_entry_count();
    if count == 0 {
        frame.render_widget(
            Paragraph::new("(No saved profiles)").style(Style::default().fg(theme.muted)),
            list_area,
        );
    } else {
        let selected = app.investigation_profiles_index();
        let lines = (app.investigation_profiles_scroll_offset()..count)
            .take(list_area.height as usize)
            .map(|index| {
                let profile = &app.runtime.saved_investigation_profiles[index];
                let is_selected = index == selected;
                let active = app
                    .active_investigation_profile
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(&profile.name));
                let style = if is_selected {
                    Style::default()
                        .fg(theme.text)
                        .bg(theme.highlight)
                        .add_modifier(Modifier::BOLD)
                } else if active {
                    Style::default()
                        .fg(theme.text)
                        .bg(theme.selection)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                Line::from(Span::styled(
                    profile_row_text(
                        if is_selected { ">" } else { " " },
                        &profile.name,
                        profile.tracked_names.len(),
                        profile.graphs.len(),
                        active,
                        list_area.width as usize,
                    ),
                    style,
                ))
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), list_area);
    }

    if let Some(profile) = app.selected_investigation_profile() {
        draw_section_label(
            frame,
            content,
            layout.summary_label_row,
            &format!("SELECTED PROFILE · {}", profile.name),
            theme,
        );
        let ma_count = profile
            .graphs
            .iter()
            .filter(|graph| {
                matches!(
                    graph.display_mode.trim().to_ascii_lowercase().as_str(),
                    "ma" | "ma5" | "moving_average_5"
                )
            })
            .count();
        let raw_count = profile.graphs.len().saturating_sub(ma_count);
        let summary = [
            format!(
                "Tracking    {} name{}    Tracked-only {}    View {}",
                profile.tracked_names.len(),
                if profile.tracked_names.len() == 1 {
                    ""
                } else {
                    "s"
                },
                on_off(profile.tracked_only),
                profile.process_view
            ),
            format!(
                "Processes   {} columns    Sort {} {}",
                profile.process_columns.len(),
                profile.sort_by,
                profile.sort_order
            ),
            format!(
                "Graphs      {} ({} Raw / {} MA5)    Layout {}    Span {}s",
                profile.graphs.len(),
                raw_count,
                ma_count,
                graph_layout_label(profile.graph_columns),
                profile.graph_time_span_seconds
            ),
            format!(
                "Inspector   Samples {}    Delta {}    Y min {}",
                on_off(profile.samples),
                on_off(profile.delta),
                if profile.y_axis_zero_min { "0" } else { "data" }
            ),
            format!("Recording   {}s", profile.recording_interval_seconds),
        ];
        for (offset, text) in summary.into_iter().enumerate() {
            frame.render_widget(
                Paragraph::new(text).style(Style::default().fg(theme.text)),
                row(content, layout.summary_row + offset as u16),
            );
        }
    } else {
        draw_section_label(
            frame,
            content,
            layout.summary_label_row,
            "SELECTED PROFILE",
            theme,
        );
        frame.render_widget(
            Paragraph::new("No saved Profiles. Press s to save the current investigation.")
                .style(Style::default().fg(theme.text)),
            row(content, layout.summary_row),
        );
    }

    frame.render_widget(
        Paragraph::new(format!(
            "Startup behavior: {}",
            app.runtime.investigation_startup.label()
        ))
        .style(Style::default().fg(theme.muted)),
        row(content, layout.startup_row),
    );

    frame.render_widget(
        Paragraph::new(Line::from(shortcut_spans(
            &[("Esc", "Close"), ("↑/↓", "Select"), ("Enter", "Load")],
            theme,
        ))),
        row(content, layout.navigation_shortcut_row),
    );
    frame.render_widget(
        Paragraph::new(Line::from(shortcut_spans(
            &[
                ("s", "Save"),
                ("S", "Save New"),
                ("u", "Startup"),
                ("F2", "Rename"),
                ("Delete", "Delete"),
            ],
            theme,
        ))),
        row(content, layout.management_shortcut_row),
    );
}

fn draw_startup(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let Some(InvestigationProfilesView::Startup { selected }) = app.investigation_profiles_view()
    else {
        return;
    };
    let popup = startup_dialog_area(area);
    let block = panel_block_focused(panel_title("STARTUP BEHAVIOR"), theme, true);
    let content = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new("Choose what to load when winproc-tui starts.")
            .style(Style::default().fg(theme.muted)),
        row(content, 0),
    );
    for (index, startup) in InvestigationStartup::ALL.into_iter().enumerate() {
        let is_selected = startup == *selected;
        let style = if is_selected {
            Style::default()
                .fg(theme.text)
                .bg(theme.highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        let text = format!(
            "{} {:<16} {}",
            if is_selected { ">" } else { " " },
            startup.label(),
            startup_description(startup)
        );
        frame.render_widget(
            Paragraph::new(text).style(style),
            row(content, STARTUP_OPTION_ROW + index as u16),
        );
    }
    frame.render_widget(
        Paragraph::new(Line::from(shortcut_spans(
            &[("↑/↓", "Select"), ("Enter", "Apply"), ("Esc", "Back")],
            theme,
        ))),
        row(content, STARTUP_SHORTCUT_ROW),
    );
}

fn draw_name_input(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let Some(InvestigationProfilesView::NameInput {
        purpose,
        draft,
        cursor,
        error,
    }) = app.investigation_profiles_view()
    else {
        return;
    };
    let popup = centered_dialog_rect(area, NAME_DIALOG_WIDTH, NAME_DIALOG_HEIGHT);
    let title = match purpose {
        ProfileNameInputPurpose::SaveAs => "SAVE INVESTIGATION PROFILE AS",
        ProfileNameInputPurpose::Rename => "RENAME INVESTIGATION PROFILE",
    };
    let block = panel_block_focused(panel_title(title), theme, true);
    let content = block.inner(popup);
    let input_area = row(content, 2);
    let (input, cursor_x) = input_view(draft, *cursor, input_area.width as usize);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new("Enter a unique profile name.").style(Style::default().fg(theme.text)),
        row(content, 0),
    );
    frame.render_widget(
        Paragraph::new(input).style(Style::default().fg(theme.text).bg(theme.panel_alt)),
        input_area,
    );
    if let Some(error) = error {
        frame.render_widget(
            Paragraph::new(error.as_str()).style(Style::default().fg(theme.danger)),
            row(content, 3),
        );
    }
    frame.render_widget(
        Paragraph::new(Line::from(shortcut_spans(
            &[("Enter", "Save"), ("Esc", "Cancel")],
            theme,
        ))),
        row(content, 5),
    );
    frame.set_cursor_position(Position::new(
        input_area.x.saturating_add(cursor_x as u16),
        input_area.y,
    ));
}

fn draw_delete_confirm(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let Some(InvestigationProfilesView::ConfirmDelete { name }) = app.investigation_profiles_view()
    else {
        return;
    };
    draw_confirm(
        frame,
        area,
        "DELETE INVESTIGATION PROFILE?",
        &format!("Delete \"{name}\"? The current setup is kept."),
        "This cannot be undone.",
        "Delete",
        theme,
    );
}

fn draw_load_confirm(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let Some(InvestigationProfilesView::ConfirmLoad { pending }) =
        app.investigation_profiles_view()
    else {
        return;
    };
    draw_confirm(
        frame,
        area,
        "LOAD INVESTIGATION PROFILE?",
        &format!(
            "This removes {} tracked name{}.",
            pending.tracking_switch.removed_name_count,
            if pending.tracking_switch.removed_name_count == 1 {
                ""
            } else {
                "s"
            }
        ),
        &format!(
            "{} older sample{} across {} name{} will be discarded.",
            pending.tracking_switch.discarded_sample_count,
            if pending.tracking_switch.discarded_sample_count == 1 {
                ""
            } else {
                "s"
            },
            pending.tracking_switch.affected_name_count,
            if pending.tracking_switch.affected_name_count == 1 {
                ""
            } else {
                "s"
            }
        ),
        "Load",
        theme,
    );
}

fn draw_confirm(
    frame: &mut ratatui::Frame<'_>,
    area: Rect,
    title: &'static str,
    message: &str,
    detail: &str,
    apply_label: &'static str,
    theme: Theme,
) {
    let popup = centered_dialog_rect(area, CONFIRM_DIALOG_WIDTH, CONFIRM_DIALOG_HEIGHT);
    let block = confirm_dialog::warning_block(title, theme);
    let content = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(message).alignment(Alignment::Center),
        row(content, 1),
    );
    frame.render_widget(
        Paragraph::new(detail)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.warning)),
        row(content, 2),
    );
    frame.render_widget(
        Paragraph::new(Line::from(warning_shortcut_spans(
            &[("Enter/Esc/n", "Cancel"), ("y", apply_label)],
            theme,
        )))
        .alignment(Alignment::Center),
        row(content, 5),
    );
}

fn draw_load_report(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let Some(InvestigationProfilesView::LoadReport {
        name,
        loaded_graph_count,
        unresolved,
    }) = app.investigation_profiles_view()
    else {
        return;
    };
    let popup = report_dialog_area(area);
    let block = panel_block_focused(panel_title("PROFILE LOAD RESULT"), theme, true);
    let content = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Loaded \"{name}\": {loaded_graph_count} Graph{}, {} unresolved.",
            if *loaded_graph_count == 1 { "" } else { "s" },
            unresolved.len()
        ))
        .style(Style::default().fg(theme.text)),
        row(content, 0),
    );
    frame.render_widget(
        Paragraph::new("Unresolved templates were not guessed or redirected.")
            .style(Style::default().fg(theme.warning)),
        row(content, 2),
    );
    let list = report_list_area(content);
    let lines = unresolved
        .iter()
        .skip(app.investigation_profiles_scroll_offset())
        .take(list.height as usize)
        .map(|line| Line::from(Span::styled(line.clone(), Style::default().fg(theme.text))))
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), list);
    frame.render_widget(
        Paragraph::new(Line::from(shortcut_spans(
            &[
                ("↑/↓", "Scroll"),
                ("PageUp/PageDown", "Page"),
                ("Enter/Esc", "Close"),
            ],
            theme,
        ))),
        row(content, REPORT_SHORTCUT_ROW),
    );
}

pub(crate) fn investigation_profiles_page_size_for_screen(area: Rect, app: &App) -> usize {
    if matches!(
        app.investigation_profiles_view(),
        Some(InvestigationProfilesView::LoadReport { .. })
    ) {
        let popup = report_dialog_area(area);
        let content = popup.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        });
        report_list_area(content).height.max(1) as usize
    } else {
        browse_layout(area, app.investigation_profiles_entry_count())
            .list
            .height
            .max(1) as usize
    }
}

pub(crate) fn investigation_profile_index_at(
    area: Rect,
    x: u16,
    y: u16,
    scroll_offset: usize,
    profile_count: usize,
) -> Option<usize> {
    let list = browse_layout(area, profile_count).list;
    if !contains(list, x, y) {
        return None;
    }
    let index = scroll_offset.saturating_add(y.saturating_sub(list.y) as usize);
    (index < profile_count).then_some(index)
}

pub(crate) fn investigation_profile_startup_at_for_screen(
    area: Rect,
    x: u16,
    y: u16,
) -> Option<InvestigationStartup> {
    let popup = startup_dialog_area(area);
    let content = popup.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    InvestigationStartup::ALL
        .into_iter()
        .enumerate()
        .find_map(|(index, startup)| {
            contains(row(content, STARTUP_OPTION_ROW + index as u16), x, y).then_some(startup)
        })
}

pub(crate) fn investigation_profile_startup_link_at_for_screen(
    area: Rect,
    x: u16,
    y: u16,
    profile_count: usize,
) -> bool {
    let layout = browse_layout(area, profile_count);
    contains(row(layout.content, layout.startup_row), x, y)
}

fn profile_row_text(
    cursor: &str,
    name: &str,
    tracked_count: usize,
    graph_count: usize,
    active: bool,
    width: usize,
) -> String {
    let suffix = format!(
        "  {:>2} tracked  {:>2} Graphs{}",
        tracked_count,
        graph_count,
        if active { "  [current]" } else { "" }
    );
    let name_width = width.saturating_sub(cursor.chars().count() + 1 + suffix.chars().count());
    format!(
        "{cursor} {:<name_width$}{suffix}",
        truncate(name, name_width)
    )
}

fn graph_layout_label(columns: u8) -> &'static str {
    match columns {
        1 => "1 col",
        2 => "2 cols",
        3 => "3 cols",
        _ => "Auto",
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "On" } else { "Off" }
}

fn startup_description(startup: InvestigationStartup) -> &'static str {
    match startup {
        InvestigationStartup::ResumeLast => "Restore the last investigation",
        InvestigationStartup::ChooseProfile => "Ask which Profile to load",
        InvestigationStartup::StartEmpty => "Use default investigation settings",
    }
}

fn draw_section_label(
    frame: &mut ratatui::Frame<'_>,
    content: Rect,
    offset: u16,
    label: &str,
    theme: Theme,
) {
    frame.render_widget(
        Paragraph::new(label.to_string()).style(
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        ),
        row(content, offset),
    );
}

fn truncate(value: &str, width: usize) -> String {
    let len = value.chars().count();
    if len <= width {
        return value.to_string();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    format!(
        "{}...",
        value
            .chars()
            .take(width.saturating_sub(3))
            .collect::<String>()
    )
}

#[derive(Clone, Copy)]
struct BrowseLayout {
    popup: Rect,
    content: Rect,
    list: Rect,
    summary_label_row: u16,
    summary_row: u16,
    startup_row: u16,
    navigation_shortcut_row: u16,
    management_shortcut_row: u16,
}

fn browse_layout(area: Rect, profile_count: usize) -> BrowseLayout {
    let list_height = (profile_count as u16).clamp(1, MAX_LIST_HEIGHT);
    let summary_label_row = LIST_ROW.saturating_add(list_height).saturating_add(1);
    let summary_row = summary_label_row.saturating_add(1);
    let startup_row = summary_row.saturating_add(6);
    let navigation_shortcut_row = startup_row.saturating_add(2);
    let management_shortcut_row = navigation_shortcut_row.saturating_add(1);
    let popup = centered_dialog_rect(
        area,
        DIALOG_WIDTH,
        management_shortcut_row.saturating_add(3),
    );
    let content = popup.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let list = Rect::new(
        content.x,
        content.y.saturating_add(LIST_ROW),
        content.width,
        list_height.min(content.height.saturating_sub(LIST_ROW)),
    );
    BrowseLayout {
        popup,
        content,
        list,
        summary_label_row,
        summary_row,
        startup_row,
        navigation_shortcut_row,
        management_shortcut_row,
    }
}

fn report_dialog_area(area: Rect) -> Rect {
    centered_dialog_rect(area, DIALOG_WIDTH, REPORT_DIALOG_HEIGHT)
}

fn startup_dialog_area(area: Rect) -> Rect {
    centered_dialog_rect(area, STARTUP_DIALOG_WIDTH, STARTUP_DIALOG_HEIGHT)
}

fn report_list_area(content: Rect) -> Rect {
    Rect::new(
        content.x,
        content.y.saturating_add(REPORT_LIST_ROW),
        content.width,
        REPORT_LIST_HEIGHT.min(content.height.saturating_sub(REPORT_LIST_ROW)),
    )
}

fn row(content: Rect, offset: u16) -> Rect {
    Rect::new(
        content.x,
        content.y.saturating_add(offset),
        content.width,
        1,
    )
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x && x < area.right() && y >= area.y && y < area.bottom()
}

fn input_view(value: &str, cursor: usize, width: usize) -> (String, usize) {
    let cursor = cursor.min(value.len());
    let cursor_char = value[..cursor].chars().count();
    let char_count = value.chars().count();
    let start_char = cursor_char.saturating_sub(width.saturating_sub(1));
    let end_char = start_char.saturating_add(width).min(char_count);
    let visible = value
        .chars()
        .skip(start_char)
        .take(end_char.saturating_sub(start_char))
        .collect::<String>();
    (visible, cursor_char.saturating_sub(start_char))
}

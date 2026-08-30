use ratatui::{
    layout::{Alignment, Rect},
    prelude::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    App,
    app::{AppActivity, SampleFreshness},
    ui::Theme,
};

const SPINNER: [char; 4] = ['|', '/', '-', '\\'];
const HEADER_ITEM_GAP: usize = 2;
const HEADER_MENU_LABEL: &str = "[MENU]";
const PROFILE_LABEL_MAX_WIDTH: u16 = 28;

pub(crate) fn draw_header(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let mut spans = Vec::new();

    let activity = app.activity();
    if header_menu_area(area, app).is_some() {
        spans.push(Span::styled(
            HEADER_MENU_LABEL,
            header_menu_style(app, theme),
        ));
        spans.push(Span::raw("  "));
    }
    spans.push(mode_span(
        activity_label(activity),
        match activity {
            AppActivity::Live => theme.active_series,
            AppActivity::Recording => theme.danger,
            AppActivity::LogView => theme.warning,
        },
        theme,
    ));
    match activity {
        AppActivity::Live => {}
        AppActivity::Recording => {
            if let Some(interval_seconds) = app
                .active_recording_interval_seconds()
                .filter(|interval| *interval > 1)
            {
                append_recording_interval(&mut spans, interval_seconds, theme);
            }
            if !app.is_display_paused() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    SPINNER[app.recording_spinner_index % SPINNER.len()].to_string(),
                    Style::default()
                        .fg(theme.danger)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        }
        AppActivity::LogView => {
            if let Some(interval_seconds) = app.log_view_interval_seconds {
                append_recording_interval(&mut spans, interval_seconds, theme);
            }
        }
    }

    if let Some(SampleFreshness::Stale { age_seconds }) = app.sample_freshness() {
        spans.push(stale_span(age_seconds, theme));
    }
    if app.is_display_paused() && activity != AppActivity::LogView {
        spans.push(Span::raw("  "));
        spans.push(mode_span("DISPLAY PAUSED", theme.warning, theme));
    }
    append_profile_label(&mut spans, app, activity, theme);
    let active_log_name = app.active_log_path().map(|path| {
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned()
    });
    let product_and_version = format!("winproc-tui {}", env!("CARGO_PKG_VERSION"));
    let product_width = product_and_version.chars().count();
    let left_content_width = spans_width(&spans).saturating_add(
        active_log_name
            .as_ref()
            .map(|name| HEADER_ITEM_GAP.saturating_add(name.chars().count()))
            .unwrap_or(0),
    );
    let show_product_and_version = left_content_width
        .saturating_add(HEADER_ITEM_GAP)
        .saturating_add(product_width)
        <= usize::from(area.width);
    let left_area = if show_product_and_version {
        Rect::new(
            area.x,
            area.y,
            area.width
                .saturating_sub(product_width as u16)
                .saturating_sub(HEADER_ITEM_GAP as u16),
            area.height,
        )
    } else {
        area
    };

    if let Some(name) = active_log_name {
        append_log_name(
            &mut spans,
            left_area,
            &name,
            if activity == AppActivity::Recording {
                theme.warning
            } else {
                theme.text
            },
        );
    }

    let header = Line::from(spans);

    let header_widget = Paragraph::new(header)
        .style(Style::default().bg(theme.panel))
        .alignment(Alignment::Left);
    frame.render_widget(header_widget, area);

    if show_product_and_version {
        let product_area = Rect::new(
            area.x
                .saturating_add(area.width.saturating_sub(product_width as u16)),
            area.y,
            product_width as u16,
            area.height,
        );
        let product_widget = Paragraph::new(product_and_version)
            .style(Style::default().fg(theme.muted).bg(theme.panel))
            .alignment(Alignment::Right);
        frame.render_widget(product_widget, product_area);
    }
}

pub(crate) fn header_menu_area(area: Rect, app: &App) -> Option<Rect> {
    let badge_width = activity_label(app.activity())
        .chars()
        .count()
        .saturating_add(2) as u16;
    let width = HEADER_MENU_LABEL.chars().count() as u16;
    let required_width = width
        .saturating_add(HEADER_ITEM_GAP as u16)
        .saturating_add(badge_width);
    (area.height > 0 && required_width <= area.width).then_some(Rect::new(area.x, area.y, width, 1))
}

pub(crate) fn header_menu_area_for_screen(area: Rect, app: &App) -> Option<Rect> {
    header_menu_area(crate::ui::layout::screen_layout(area)[0], app)
}

fn activity_label(activity: AppActivity) -> &'static str {
    match activity {
        AppActivity::Live => "LIVE",
        AppActivity::Recording => "REC",
        AppActivity::LogView => "LOG",
    }
}

fn header_menu_style(app: &App, theme: Theme) -> Style {
    if app.is_main_menu_open() {
        Style::default()
            .fg(theme.text)
            .bg(theme.table_selection_surface)
            .add_modifier(Modifier::BOLD)
    } else if app.header_menu_hovered {
        Style::default()
            .fg(theme.text)
            .bg(theme.focus_surface)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.key_hint)
            .bg(theme.panel)
            .add_modifier(Modifier::BOLD)
    }
}

fn append_recording_interval(spans: &mut Vec<Span<'static>>, seconds: u64, theme: Theme) {
    spans.push(Span::raw(" · "));
    spans.push(Span::styled(
        if seconds > 1 {
            format!("{seconds}s AVG")
        } else {
            "1s".to_string()
        },
        Style::default()
            .fg(theme.muted)
            .add_modifier(Modifier::BOLD),
    ));
}

fn stale_span(age_seconds: u64, theme: Theme) -> Span<'static> {
    Span::styled(
        format!(" · STALE {age_seconds}s"),
        Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD),
    )
}

fn append_profile_label(
    spans: &mut Vec<Span<'static>>,
    app: &App,
    activity: AppActivity,
    theme: Theme,
) {
    let label = match activity {
        AppActivity::LogView => "PF: --".to_string(),
        AppActivity::Live | AppActivity::Recording => {
            let Some(name) = app.active_investigation_profile.as_deref() else {
                append_profile_badge(spans, "PF: none", theme);
                return;
            };
            let modified = app.active_investigation_profile_dirty();
            format!("PF: {name}{}", if modified { "*" } else { "" })
        }
    };
    append_profile_badge(spans, &label, theme);
}

fn append_profile_badge(spans: &mut Vec<Span<'static>>, label: &str, theme: Theme) {
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        truncate_middle(label, PROFILE_LABEL_MAX_WIDTH),
        Style::default().fg(Color::Black).bg(theme.muted),
    ));
}

fn append_log_name(
    spans: &mut Vec<Span<'static>>,
    area: Rect,
    name: &str,
    color: ratatui::prelude::Color,
) {
    let used_width = spans_width(spans);
    let name_width =
        usize::from(area.width).saturating_sub(used_width.saturating_add(HEADER_ITEM_GAP));
    if name_width == 0 {
        return;
    }
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        truncate_middle(name, name_width.min(usize::from(u16::MAX)) as u16),
        Style::default().fg(color),
    ));
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}

fn mode_span(label: &'static str, color: ratatui::prelude::Color, theme: Theme) -> Span<'static> {
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(theme.background)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

fn truncate_middle(value: &str, max_width: u16) -> String {
    let max_width = max_width as usize;
    let char_count = value.chars().count();
    if char_count <= max_width {
        return value.to_string();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }

    let tail_len = (max_width / 2).max(1);
    let head_len = max_width.saturating_sub(tail_len + 3);
    let head = value.chars().take(head_len).collect::<String>();
    let tail = value
        .chars()
        .rev()
        .take(tail_len)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}...{tail}")
}

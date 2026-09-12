use crate::{
    App,
    app::{AppActivity, ProcessInfoFocus},
    model::scheduling::{PriorityClass, PriorityOutcome},
    ui::Theme,
};
use ratatui::{
    layout::Rect,
    prelude::{Modifier, Style},
    text::{Line, Span},
};

pub(crate) const OPTION_START: usize = 3;

pub(crate) fn option_at(app: &App, area: Rect, x: u16, y: u16) -> Option<usize> {
    if !area.contains((x, y).into())
        || x == area.right().saturating_sub(1)
        || app.activity() == AppActivity::LogView
    {
        return None;
    }
    let line = app.scheduling.scroll.offset + usize::from(y - area.y);
    line.checked_sub(OPTION_START)
        .filter(|index| *index < PriorityClass::EDITABLE.len())
}

pub(crate) fn lines(app: &App, width: u16, theme: Theme) -> Vec<Line<'static>> {
    if app.activity() == AppActivity::LogView {
        return vec![Line::from("Not recorded in Log view.")];
    }
    if let Some(change) = app.scheduling.confirmation {
        let mut text = vec![
            if change.restore {
                "Restore previous priority?".into()
            } else {
                "Change priority?".into()
            },
            format!("Current:  {}", change.expected.label()),
            format!("Proposed: {}", change.desired.label()),
        ];
        if change.desired == PriorityClass::High {
            text.push("High can reduce responsiveness of other processes.".into());
        }
        if !change.expected.editable() {
            text.push("The current priority cannot be restored by this editor.".into());
        }
        return text
            .into_iter()
            .flat_map(|text| super::process_info_dialog::wrap_display_width(&text, width as usize))
            .map(Line::from)
            .collect();
    }
    let report = app.scheduling.report.as_ref();
    let current = report
        .and_then(|report| report.current)
        .map(PriorityClass::label)
        .unwrap_or_else(|| "--".into());
    let mut result = vec![
        Line::from(format!("Current priority: {current}")),
        Line::from(if app.scheduling.applying {
            "Applying..."
        } else if app.scheduling.pending {
            "Reading..."
        } else if report
            .is_some_and(|report| matches!(report.outcome, PriorityOutcome::AppliedUnverified(_)))
        {
            "Applied; verification incomplete"
        } else if report.is_some_and(|report| matches!(report.outcome, PriorityOutcome::Failed(_)))
        {
            "Priority request failed"
        } else if report.is_some_and(|report| report.writable) {
            "Access: Read/write"
        } else if report.is_none() {
            "Access: --"
        } else {
            "Access: Read-only"
        }),
        Line::default(),
    ];
    for (index, class) in PriorityClass::EDITABLE.into_iter().enumerate() {
        let selected = index == app.scheduling.selected;
        let style = if selected && app.process_info_focus == ProcessInfoFocus::Content {
            Style::default()
                .fg(theme.text)
                .bg(theme.focus_surface)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        result.push(Line::from(Span::styled(
            format!("({}) {}", if selected { '●' } else { ' ' }, class.label()),
            style,
        )));
    }
    let previous = report
        .and_then(|report| report.restore)
        .map(|change| change.desired.label())
        .unwrap_or_else(|| "--".into());
    result.push(Line::from(format!("Previous priority: {previous}")));
    let outcome = report.and_then(|report| match &report.outcome {
        PriorityOutcome::Read => None,
        PriorityOutcome::Changed => Some("Priority changed and verified."),
        PriorityOutcome::Restored => Some("Previous priority restored and verified."),
        PriorityOutcome::Failed(error) | PriorityOutcome::AppliedUnverified(error) => {
            Some(error.as_str())
        }
    });
    for message in [
        if app.scheduling.pending {
            None
        } else {
            app.scheduling_disabled_reason()
        },
        outcome,
        app.scheduling.notice.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        result.extend(
            super::process_info_dialog::wrap_display_width(message, width.max(1) as usize)
                .into_iter()
                .map(|line| Line::from(Span::styled(line, Style::default().fg(theme.warning)))),
        );
    }
    result
}

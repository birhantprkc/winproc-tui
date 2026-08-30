use ratatui::{
    prelude::{Modifier, Style},
    text::Span,
};

use crate::{app::GraphSourceState, ui::Theme};

pub(crate) fn graph_value_style(
    base: Style,
    state: Option<GraphSourceState>,
    theme: Theme,
) -> Style {
    let Some(state) = state else {
        return base;
    };
    let style = base
        .fg(theme.active_series)
        .remove_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED);
    if state.active {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

pub(crate) fn graph_value_spans(
    value: impl Into<String>,
    base: Style,
    state: Option<GraphSourceState>,
    theme: Theme,
) -> Vec<Span<'static>> {
    let value = value.into();
    let token = value.trim();
    if token.is_empty() {
        return vec![Span::styled(value, base)];
    }

    let token_start = value.find(token).unwrap_or_default();
    let token_end = token_start + token.len();
    let mut spans = Vec::with_capacity(3);
    if token_start > 0 {
        spans.push(Span::styled(value[..token_start].to_string(), base));
    }
    spans.push(Span::styled(
        value[token_start..token_end].to_string(),
        graph_value_style(base, state, theme),
    ));
    if token_end < value.len() {
        spans.push(Span::styled(value[token_end..].to_string(), base));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    #[test]
    fn registered_values_use_underlining_with_bold_reserved_for_the_active_graph() {
        for theme in crate::ui::THEMES {
            let base = Style::default()
                .bg(theme.focus_surface)
                .add_modifier(Modifier::BOLD);
            let inactive = graph_value_style(
                base,
                Some(GraphSourceState {
                    ordinal: 0,
                    active: false,
                }),
                theme,
            );
            let active = graph_value_style(
                base,
                Some(GraphSourceState {
                    ordinal: 1,
                    active: true,
                }),
                theme,
            );

            assert_eq!(inactive.fg, Some(theme.active_series));
            assert_eq!(inactive.bg, Some(theme.focus_surface));
            assert!(inactive.add_modifier.contains(Modifier::UNDERLINED));
            assert!(!inactive.add_modifier.contains(Modifier::BOLD));
            assert_eq!(active.fg, Some(theme.active_series));
            assert_eq!(active.bg, Some(theme.focus_surface));
            assert!(active.add_modifier.contains(Modifier::UNDERLINED));
            assert!(active.add_modifier.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn unregistered_value_keeps_its_base_style() {
        for theme in crate::ui::THEMES {
            let base = Style::default()
                .fg(theme.text)
                .bg(theme.panel_alt)
                .add_modifier(Modifier::UNDERLINED);
            let style = graph_value_style(base, None, theme);

            assert_eq!(style, base);
        }
    }

    #[test]
    fn graph_value_spans_exclude_alignment_padding_from_registration_style() {
        for theme in crate::ui::THEMES {
            let base = Style::default()
                .fg(theme.text)
                .bg(theme.table_selection_surface);
            let spans = graph_value_spans(
                "  -- ",
                base,
                Some(GraphSourceState {
                    ordinal: 0,
                    active: false,
                }),
                theme,
            );

            assert_eq!(spans.len(), 3);
            assert_eq!(spans[0].content.as_ref(), "  ");
            assert_eq!(spans[0].style, base);
            assert_eq!(spans[1].content.as_ref(), "--");
            assert_eq!(spans[1].style.fg, Some(theme.active_series));
            assert_eq!(spans[1].style.bg, Some(theme.table_selection_surface));
            assert!(spans[1].style.add_modifier.contains(Modifier::UNDERLINED));
            assert_eq!(spans[2].content.as_ref(), " ");
            assert_eq!(spans[2].style, base);
        }
    }
}

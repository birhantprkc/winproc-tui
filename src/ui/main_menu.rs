use ratatui::{
    layout::Rect,
    prelude::{Modifier, Style},
    text::{Line, Span, Text},
};

use crate::{
    App,
    app::state::{MainMenuItem, MainMenuRow},
    ui::{Theme, widgets::scrollable_modal::ScrollableModal},
};

const HEADER_HEIGHT: u16 = 1;

pub(crate) fn draw_main_menu(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App, theme: Theme) {
    let content_width = usize::from(main_menu_content_width(app));
    let lines = app
        .main_menu_rows()
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let cursor = if index == app.main_menu_selected {
                "> "
            } else {
                "  "
            };
            let style = if index == app.main_menu_selected {
                Style::default()
                    .fg(theme.text)
                    .bg(theme.table_selection_surface)
                    .add_modifier(Modifier::BOLD)
            } else if app.main_menu_hovered == Some(index) {
                Style::default()
                    .fg(theme.text)
                    .bg(theme.focus_surface)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            let indent = "  ".repeat(usize::from(row.depth));
            let label = format!("{cursor}{indent}{}", app.main_menu_row_label(*row));
            Line::from(Span::styled(format!("{label:<content_width$}"), style))
        })
        .collect::<Vec<_>>();

    main_menu_modal(app).render(frame, area, Text::from(lines), 0, false, theme);
}

pub(crate) fn main_menu_index_at(area: Rect, app: &App, x: u16, y: u16) -> Option<usize> {
    let content = main_menu_modal(app).layout(area).content;
    if x < content.x || x >= content.right() || y < content.y || y >= content.bottom() {
        return None;
    }
    let index = usize::from(y - content.y);
    (index < app.main_menu_rows().len()).then_some(index)
}

#[cfg(test)]
pub(crate) fn main_menu_area(area: Rect, app: &App) -> Rect {
    main_menu_modal(app).area(area)
}

#[cfg(test)]
pub(crate) fn main_menu_item_area(area: Rect, app: &App, index: usize) -> Option<Rect> {
    let content = main_menu_modal(app).layout(area).content;
    (index < app.main_menu_rows().len() && index < usize::from(content.height)).then(|| {
        Rect::new(
            content.x,
            content.y.saturating_add(index as u16),
            content.width,
            1,
        )
    })
}

fn main_menu_modal(app: &App) -> ScrollableModal {
    ScrollableModal::new(
        "",
        main_menu_content_width(app),
        app.main_menu_rows().len().min(usize::from(u16::MAX)) as u16,
        0,
    )
    .with_top_left_placement(HEADER_HEIGHT)
}

fn main_menu_content_width(app: &App) -> u16 {
    let activity = app.activity();
    activity
        .main_menu_items()
        .iter()
        .flat_map(|item| {
            let root = MainMenuRow {
                item: *item,
                depth: 0,
            };
            let children = match item {
                MainMenuItem::Section(section) => section.actions(activity),
                MainMenuItem::Action(_) => &[],
            };
            std::iter::once(root).chain(children.iter().copied().map(|action| MainMenuRow {
                item: MainMenuItem::Action(action),
                depth: 1,
            }))
        })
        .map(|row| 2 + usize::from(row.depth) * 2 + app.main_menu_row_label(row).chars().count())
        .max()
        .unwrap_or_default()
        .min(usize::from(u16::MAX)) as u16
}

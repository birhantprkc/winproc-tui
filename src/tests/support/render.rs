use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use crate::app::App;
use crate::ui::{self, main_panel_areas_for_app};

pub(in crate::tests) fn render_app_to_text(app: &App, width: u16, height: u16) -> String {
    buffer_to_text(&render_app_to_buffer(app, width, height))
}

pub(in crate::tests) fn render_app_to_buffer(
    app: &App,
    width: u16,
    height: u16,
) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal should be created");
    terminal
        .draw(|frame| ui::draw(frame, app))
        .expect("test render should succeed");
    terminal.backend().buffer().clone()
}

pub(in crate::tests) fn left_click(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

pub(in crate::tests) fn mouse_move(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Moved,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

pub(in crate::tests) fn assert_modal_rect_focus_border(app: &App, popup: Rect) {
    let screen = Rect::new(0, 0, 100, 45);
    let buffer = render_app_to_buffer(app, screen.width, screen.height);
    let process_table = main_panel_areas_for_app(screen, app).processes.area;
    let theme = app.theme();

    assert_eq!(
        buffer[(popup.x, popup.y)].fg,
        theme.focus_border,
        "modal border should use the high-contrast neutral focus color"
    );
    assert_eq!(
        buffer[(process_table.x, process_table.y)].symbol(),
        "╭",
        "underlying process table should not stay focused while a modal is open"
    );
    assert_ne!(
        buffer[(process_table.x, process_table.y)].fg,
        theme.border,
        "underlying process table border should be dimmed while a modal is open"
    );
}

pub(in crate::tests) fn buffer_to_text(buffer: &ratatui::buffer::Buffer) -> String {
    buffer
        .content()
        .chunks(buffer.area().width as usize)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(in crate::tests) fn assert_dialog_title_style(
    buffer: &ratatui::buffer::Buffer,
    title: &str,
    theme: ui::Theme,
) {
    assert_title_style(
        buffer,
        title,
        theme.focus_border,
        ui::theme::contrasting_foreground(theme.focus_border, theme),
    );
}

pub(in crate::tests) fn assert_title_style(
    buffer: &ratatui::buffer::Buffer,
    title: &str,
    expected_background: ratatui::style::Color,
    expected_foreground: ratatui::style::Color,
) {
    let (x, y) = find_text_position(buffer, title)
        .unwrap_or_else(|| panic!("dialog title should render: {title}"));
    let cell = &buffer[(x, y)];
    assert_eq!(cell.fg, expected_foreground, "dialog title: {title}");
    assert_eq!(cell.bg, expected_background, "dialog title: {title}");
    assert!(
        cell.modifier.contains(Modifier::BOLD),
        "dialog title should be bold: {title}"
    );
}

pub(in crate::tests) fn find_text_position(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
) -> Option<(u16, u16)> {
    let width = buffer.area().width;
    let height = buffer.area().height;
    for y in 0..height {
        let row = (0..width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>();
        if let Some(x) = row.find(needle) {
            return Some((row[..x].chars().count() as u16, y));
        }
    }
    None
}

pub(in crate::tests) fn assert_blank_row_above_text(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
) {
    let (x, y) = find_text_position(buffer, needle)
        .unwrap_or_else(|| panic!("shortcut guidance should render: {needle}"));
    assert!(y > 0, "shortcut guidance has no preceding row: {needle}");
    for offset in 0..needle.chars().count() as u16 {
        assert_eq!(
            buffer[(x + offset, y - 1)].symbol(),
            " ",
            "row above shortcut guidance is not blank: {needle}"
        );
    }
}

pub(in crate::tests) fn find_text_position_in_area(
    buffer: &ratatui::buffer::Buffer,
    area: Rect,
    needle: &str,
) -> Option<(u16, u16)> {
    let right = area.right().min(buffer.area().right());
    let bottom = area.bottom().min(buffer.area().bottom());
    for y in area.y..bottom {
        let row = (area.x..right)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>();
        if let Some(x) = row.find(needle) {
            return Some((area.x + row[..x].chars().count() as u16, y));
        }
    }
    None
}

pub(in crate::tests) fn area_contains_foreground(
    buffer: &ratatui::buffer::Buffer,
    area: Rect,
    foreground: ratatui::style::Color,
) -> bool {
    let right = area.right().min(buffer.area().right());
    let bottom = area.bottom().min(buffer.area().bottom());
    (area.y..bottom).any(|y| (area.x..right).any(|x| buffer[(x, y)].fg == foreground))
}

pub(in crate::tests) fn find_styled_symbol_positions_in_area(
    buffer: &ratatui::buffer::Buffer,
    area: Rect,
    symbol: &str,
    fg: ratatui::style::Color,
) -> Vec<(u16, u16)> {
    let right = area.right().min(buffer.area().right());
    let bottom = area.bottom().min(buffer.area().bottom());
    let mut positions = Vec::new();
    for y in area.y..bottom {
        for x in area.x..right {
            let cell = &buffer[(x, y)];
            if cell.symbol() == symbol && cell.fg == fg {
                positions.push((x, y));
            }
        }
    }
    positions
}

pub(in crate::tests) fn find_symbol_position(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
) -> Option<(u16, u16)> {
    let width = buffer.area().width;
    let height = buffer.area().height;
    for y in 0..height {
        for x in 0..width {
            if buffer[(x, y)].symbol() == needle {
                return Some((x, y));
            }
        }
    }
    None
}

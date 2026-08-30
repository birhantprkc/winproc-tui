use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::{Color, Modifier},
    widgets::Widget,
};

use crate::ui::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModalScrimStrength {
    Menu,
    Dialog,
    Priority,
}

impl ModalScrimStrength {
    const fn percent(self) -> u16 {
        match self {
            Self::Menu => 25,
            Self::Dialog => 52,
            Self::Priority => 64,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModalScrim {
    theme: Theme,
    strength: ModalScrimStrength,
}

impl ModalScrim {
    pub(crate) const fn new(theme: Theme, strength: ModalScrimStrength) -> Self {
        Self { theme, strength }
    }
}

impl Widget for ModalScrim {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                let Some(cell) = buffer.cell_mut((x, y)) else {
                    continue;
                };
                cell.fg = blend_toward(
                    cell.fg,
                    self.theme.background,
                    self.strength.percent(),
                    self.theme.muted,
                );
                cell.bg = blend_toward(
                    cell.bg,
                    self.theme.background,
                    self.strength.percent(),
                    self.theme.background,
                );
                cell.modifier.remove(Modifier::BOLD);
            }
        }
    }
}

fn blend_toward(color: Color, target: Color, percent: u16, fallback: Color) -> Color {
    let (Color::Rgb(red, green, blue), Color::Rgb(target_red, target_green, target_blue)) =
        (color, target)
    else {
        return fallback;
    };
    Color::Rgb(
        blend_channel(red, target_red, percent),
        blend_channel(green, target_green, percent),
        blend_channel(blue, target_blue, percent),
    )
}

fn blend_channel(value: u8, target: u8, percent: u16) -> u8 {
    let retained = 100_u16.saturating_sub(percent.min(100));
    ((u16::from(value) * retained + u16::from(target) * percent.min(100) + 50) / 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{prelude::Style, widgets::Paragraph};

    #[test]
    fn scrim_preserves_symbols_but_dims_colors_and_bold_weight() {
        let theme = crate::ui::THEMES[0];
        let area = Rect::new(0, 0, 4, 1);
        let mut buffer = Buffer::empty(area);
        Paragraph::new("TEST")
            .style(
                Style::default()
                    .fg(theme.focus_border)
                    .bg(theme.panel)
                    .add_modifier(Modifier::BOLD),
            )
            .render(area, &mut buffer);

        ModalScrim::new(theme, ModalScrimStrength::Dialog).render(area, &mut buffer);

        assert_eq!(buffer[(0, 0)].symbol(), "T");
        assert_ne!(buffer[(0, 0)].fg, theme.focus_border);
        assert_ne!(buffer[(0, 0)].bg, theme.panel);
        assert!(!buffer[(0, 0)].modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn stronger_scrims_move_colors_closer_to_the_background() {
        let theme = crate::ui::THEMES[0];
        let color = Color::Rgb(200, 180, 160);
        let menu = blend_toward(
            color,
            theme.background,
            ModalScrimStrength::Menu.percent(),
            theme.muted,
        );
        let priority = blend_toward(
            color,
            theme.background,
            ModalScrimStrength::Priority.percent(),
            theme.muted,
        );

        let Color::Rgb(menu_red, _, _) = menu else {
            panic!("menu color should stay RGB");
        };
        let Color::Rgb(priority_red, _, _) = priority else {
            panic!("priority color should stay RGB");
        };
        assert!(priority_red < menu_red);
    }
}

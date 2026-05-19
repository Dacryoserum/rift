use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear},
    Frame,
};

/// Centered popup. Clears background, draws a bordered block.
/// Returns inner area for content.
pub fn centered_popup(
    f: &mut Frame,
    title: &str,
    width_pct: u16,
    height_pct: u16,
    border_style: Style,
) -> Rect {
    let popup_area = centered_rect(width_pct, height_pct, f.area());
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    inner
}

pub fn centered_rect(width_pct: u16, height_pct: u16, r: Rect) -> Rect {
    let popup_height = r.height * height_pct / 100;
    let popup_width = r.width * width_pct / 100;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(popup_height)) / 2),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(popup_width)) / 2),
            Constraint::Length(popup_width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

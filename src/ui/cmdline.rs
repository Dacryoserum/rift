use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Mode};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let line = if let Some(ref err) = app.cmdline_error {
        Line::from(Span::styled(
            format!(" Error: {}", err),
            Style::default().fg(Color::Red),
        ))
    } else {
        match app.mode {
            Mode::SearchForward => {
                let text = format!("/{}\u{2588}", app.cmdline_input);
                Line::from(Span::styled(
                    text,
                    Style::default().fg(theme.cmdline_fg).bg(theme.cmdline_bg),
                ))
            }
            Mode::SearchBackward => {
                let text = format!("?{}\u{2588}", app.cmdline_input);
                Line::from(Span::styled(
                    text,
                    Style::default().fg(theme.cmdline_fg).bg(theme.cmdline_bg),
                ))
            }
            Mode::Command => {
                let text = format!(":{}\u{2588}", app.cmdline_input);
                Line::from(Span::styled(
                    text,
                    Style::default().fg(theme.cmdline_fg).bg(theme.cmdline_bg),
                ))
            }
            Mode::Normal => Line::from(Span::styled(
                " q quit | / search | ? backward | : command | h help",
                Style::default().fg(Color::DarkGray),
            )),
            _ => Line::from(""),
        }
    };

    let para = Paragraph::new(line)
        .style(Style::default().bg(theme.cmdline_bg));
    f.render_widget(para, area);
}

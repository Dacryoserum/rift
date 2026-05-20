pub mod cmdline;
pub mod minimap;
pub mod popup;
pub mod stats;
pub mod statusbar;
pub mod viewport;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Mode};

const MINIMAP_WIDTH: u16 = 3;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // ── Outer layout: content | statusbar | cmdline ────────────────────────
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let content_area = outer[0];
    let statusbar_area = outer[1];
    let cmdline_area = outer[2];

    // ── Content: viewport(s) | minimap ────────────────────────────────────
    let (viewport_area, minimap_area) = if app.show_minimap && app.config.minimap_enabled {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(10), Constraint::Length(MINIMAP_WIDTH)])
            .split(content_area);
        (split[0], Some(split[1]))
    } else {
        (content_area, None)
    };

    // ── Viewport(s): single or split ──────────────────────────────────────
    let num_panes = app.panes.len();
    if num_panes == 2 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(viewport_area);
        viewport::render(f, split[0], app, 0);
        viewport::render(f, split[1], app, 1);
    } else {
        viewport::render(f, viewport_area, app, 0);
    }

    // ── Minimap ────────────────────────────────────────────────────────────
    if let Some(mm_area) = minimap_area {
        minimap::render(f, mm_area, app);
    }

    // ── Status bar ────────────────────────────────────────────────────────
    statusbar::render(f, statusbar_area, app);

    // ── Command line ───────────────────────────────────────────────────────
    cmdline::render(f, cmdline_area, app);

    // ── Popups / overlays ─────────────────────────────────────────────────
    match app.mode {
        Mode::StatsPanel => {
            stats::render(f, app);
        }
        Mode::Help => {
            render_help(f, app);
        }
        Mode::BookmarkManager => {
            render_bookmark_manager(f, app);
        }
        Mode::FuzzySearch => {
            render_fuzzy(f, app);
        }
        _ => {}
    }
}

fn render_help(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let border_style = Style::default().fg(theme.popup_border_fg);
    let inner = popup::centered_popup(f, "Help", 70, 80, border_style);

    let help_text = vec![
        (
            "Navigation",
            vec![
                ("j / ↓", "Scroll down one line"),
                ("k / ↑", "Scroll up one line"),
                ("Ctrl+D", "Half page down"),
                ("Ctrl+U", "Half page up"),
                ("Ctrl+F / PgDn", "Full page down"),
                ("Ctrl+B / PgUp", "Full page up"),
                ("gg", "Go to first line"),
                ("G", "Go to last line"),
                ("zz / zt / zb", "Center / top / bottom cursor"),
                ("{n}G", "Jump to line n"),
                ("0", "Reset horizontal scroll"),
            ],
        ),
        (
            "Search",
            vec![
                ("/", "Search forward"),
                ("?", "Search backward"),
                ("n", "Next match"),
                ("N", "Previous match"),
                ("F", "Fuzzy search"),
            ],
        ),
        (
            "View",
            vec![
                ("l", "Toggle line numbers"),
                ("L", "Cycle line number mode"),
                ("~", "Line length bar mode"),
                ("w", "Toggle wrap"),
                ("Ctrl+W", "Split pane"),
                ("Tab", "Switch pane"),
                ("S", "Statistics"),
            ],
        ),
        (
            "Bookmarks",
            vec![
                ("m{char}", "Set bookmark"),
                ("'{char}", "Jump to bookmark"),
                ("B", "Bookmark manager (↑↓ navigate)"),
            ],
        ),
        (
            "Other",
            vec![
                ("y", "Yank current line"),
                ("V", "Visual selection"),
                ("f", "Toggle follow mode"),
                (":", "Command mode"),
                ("Esc", "Clear search highlights"),
                ("q", "Quit"),
            ],
        ),
    ];

    let label_style = Style::default()
        .fg(theme.json_key_fg)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(theme.log_info_fg);
    let desc_style = Style::default().fg(theme.foreground);

    let mut lines: Vec<Line> = Vec::new();

    for (section, items) in &help_text {
        lines.push(Line::from(Span::styled(
            format!("  {}", section),
            label_style,
        )));
        for (key, desc) in items {
            lines.push(Line::from(vec![
                Span::styled(format!("    {:16}", key), key_style),
                Span::styled(*desc, desc_style),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Press Escape or q to close",
        Style::default().fg(Color::DarkGray),
    )));

    let para =
        Paragraph::new(lines).style(Style::default().bg(theme.popup_bg).fg(theme.foreground));
    f.render_widget(para, inner);
}

fn render_bookmark_manager(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let border_style = Style::default().fg(theme.popup_border_fg);
    let inner = popup::centered_popup(f, "Bookmarks", 60, 60, border_style);

    let label_style = Style::default()
        .fg(theme.json_key_fg)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(theme.foreground);
    let hint_style = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "  Key    Line    Byte Offset",
        label_style,
    )));
    lines.push(Line::from("  ─────────────────────────────"));

    let mut marks: Vec<(char, &crate::bookmarks::Bookmark)> = app.bookmarks.all().collect();
    marks.sort_by_key(|(c, _)| *c);

    if marks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No bookmarks set. Use m{char} to set one.",
            hint_style,
        )));
    } else {
        for (i, (key, bm)) in marks.iter().enumerate() {
            let is_selected = i == app.bookmark_selected;
            let row_style = if is_selected {
                Style::default()
                    .fg(theme.foreground)
                    .bg(theme.current_line_bg)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                value_style
            };
            let prefix = if is_selected { "▶ " } else { "  " };
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "{}{:4}   {:7} {:12}",
                    prefix,
                    key,
                    bm.line_num + 1,
                    bm.byte_offset
                ),
                row_style,
            )]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑↓/jk: navigate | Enter: jump | d: delete | Esc: close",
        hint_style,
    )));

    let para =
        Paragraph::new(lines).style(Style::default().bg(theme.popup_bg).fg(theme.foreground));
    f.render_widget(para, inner);
}

fn render_fuzzy(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let border_style = Style::default().fg(theme.popup_border_fg);
    let inner = popup::centered_popup(f, "Fuzzy Search", 70, 70, border_style);

    let fuzzy = match &app.fuzzy_popup {
        Some(s) => s,
        None => return,
    };

    let hint_style = Style::default().fg(Color::DarkGray);
    let selected_style = Style::default()
        .bg(theme.current_line_bg)
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(theme.foreground);
    let match_style = Style::default().fg(theme.search_highlight_bg);

    let mut lines: Vec<Line> = Vec::new();

    // Query line
    let query_line = format!("  > {}\u{2588}", fuzzy.query);
    lines.push(Line::from(Span::styled(
        query_line,
        Style::default().fg(theme.cmdline_fg),
    )));
    lines.push(Line::from(Span::styled(
        format!("  {} matches", fuzzy.results.len()),
        hint_style,
    )));
    lines.push(Line::from(""));

    let visible_height = inner.height.saturating_sub(4) as usize;

    for (i, fm) in fuzzy
        .results
        .iter()
        .skip(fuzzy.scroll_offset)
        .take(visible_height)
        .enumerate()
    {
        let is_selected = fuzzy.scroll_offset + i == fuzzy.selected;
        let style = if is_selected {
            selected_style
        } else {
            normal_style
        };

        let line_label = format!("  {:6} ", fm.line_num + 1);
        // Highlight matched positions
        let mut spans = vec![Span::styled(line_label, hint_style)];

        let chars: Vec<char> = fm.line_text.chars().collect();
        let match_set: std::collections::HashSet<usize> = fm.indices.iter().copied().collect();
        let mut j = 0;
        while j < chars.len() {
            if match_set.contains(&j) {
                spans.push(Span::styled(chars[j].to_string(), match_style.patch(style)));
                j += 1;
            } else {
                let start = j;
                while j < chars.len() && !match_set.contains(&j) {
                    j += 1;
                }
                let text: String = chars[start..j].iter().collect();
                spans.push(Span::styled(text, style));
            }
        }

        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑↓ navigate | Enter jump | Escape close",
        hint_style,
    )));

    let para =
        Paragraph::new(lines).style(Style::default().bg(theme.popup_bg).fg(theme.foreground));
    f.render_widget(para, inner);
}

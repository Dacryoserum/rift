use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;

/// Braille density characters
fn density_char(count: u64) -> char {
    match count {
        0 => ' ',
        1 => '\u{2802}', // ⠂
        2..=3 => '\u{2806}', // ⠆
        4..=7 => '\u{2816}', // ⠖
        8..=15 => '\u{2836}', // ⠶
        _ => '\u{28F6}', // ⣶
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let height = area.height as usize;
    let file_size = app.reader.file_size.max(1);
    let _bytes_per_row = file_size / height as u64 + 1;

    let theme = &app.theme;

    // Get viewport range
    let pane = &app.panes[app.active_pane];
    let viewport_start = pane.scroll_offset;
    let viewport_end = viewport_start + pane.visible_height as u64;

    // Get total lines for bookmark mapping
    let total_lines = {
        let idx = app.line_index.read().unwrap();
        idx.line_count().max(1)
    };

    // Build density buckets from search results
    let mut hit_density = vec![0u64; height];
    for result in &app.search_results {
        let row = (result.byte_offset * height as u64 / file_size) as usize;
        if row < height {
            hit_density[row] += 1;
        }
    }

    // Bookmark rows
    let mut bookmark_rows = std::collections::HashSet::new();
    for (_, bm) in app.bookmarks.all() {
        let row = (bm.byte_offset * height as u64 / file_size) as usize;
        if row < height {
            bookmark_rows.insert(row);
        }
    }

    // Viewport rows
    let vp_start_row = (viewport_start * height as u64 / total_lines).min(height as u64 - 1) as usize;
    let vp_end_row = (viewport_end * height as u64 / total_lines).min(height as u64 - 1) as usize;

    let mut lines: Vec<Line> = Vec::with_capacity(height);

    for row in 0..height {
        let is_viewport = row >= vp_start_row && row <= vp_end_row;
        let is_bookmark = bookmark_rows.contains(&row);
        let hits = hit_density[row];

        let ch = density_char(hits);

        let style = if is_bookmark {
            Style::default().fg(theme.minimap_bookmark_fg)
        } else if is_viewport {
            Style::default().fg(theme.minimap_viewport_fg)
        } else if hits > 0 {
            Style::default().fg(theme.minimap_hit_fg)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let prefix = if is_viewport { '▐' } else { ' ' };
        let text = format!("{}{}", prefix, ch);

        lines.push(Line::from(Span::styled(text, style)));
    }

    let para = Paragraph::new(lines);
    f.render_widget(para, area);
}

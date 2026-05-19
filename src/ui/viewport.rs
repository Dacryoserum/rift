use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, LineNumberMode};
use crate::reader::index::LinePosition;

const GUTTER_WIDTH: usize = 7; // "123456 " or "~12345 "

pub fn render(f: &mut Frame, area: Rect, app: &App, pane_idx: usize) {
    if area.height == 0 {
        return;
    }

    let pane = &app.panes[pane_idx];
    let theme = &app.theme;
    let scroll = pane.scroll_offset;
    let h_offset = pane.horizontal_offset;
    let visible_height = area.height as usize;
    let visible_width = area.width as usize;

    let gutter_w = if app.show_line_numbers { GUTTER_WIDTH } else { 0 };
    let content_width = visible_width.saturating_sub(gutter_w);

    // Build bookmark set for quick lookup
    let bookmarked_lines: std::collections::HashSet<u64> =
        app.bookmarks.all().map(|(_, bm)| bm.line_num).collect();

    // Build search hit map for this view (line_num -> (start, end))
    let search_hits: std::collections::HashMap<u64, (usize, usize)> = app
        .search_results
        .iter()
        .map(|r| (r.line_num, (r.match_start, r.match_end)))
        .collect();

    let mut render_lines: Vec<Line> = Vec::with_capacity(visible_height);

    for row in 0..visible_height {
        let line_num = scroll + row as u64;

        // Get byte offset
        let (byte_offset, is_estimated) = {
            let idx = app.line_index.read().unwrap();
            match idx.offset_for_line(line_num) {
                LinePosition::Exact { byte_offset, .. } => (byte_offset, false),
                LinePosition::Estimated { byte_offset, .. } => (byte_offset, true),
            }
        };

        if byte_offset >= app.reader.file_size && app.reader.file_size > 0 {
            render_lines.push(Line::from("~"));
            continue;
        }

        // Read line bytes
        let (line_bytes, _) = app
            .reader
            .line_bytes_at(byte_offset, app.config.max_line_bytes);

        // Decode
        let line_str = app.reader.decode(line_bytes);
        let line_str: &str = &line_str;

        // Apply horizontal offset
        let display_str: String = line_str
            .chars()
            .skip(h_offset)
            .take(content_width + 1) // +1 to detect truncation
            .collect();

        let truncated = display_str.chars().count() > content_width;
        let display_str: String = display_str.chars().take(content_width).collect();

        // Build spans
        let mut spans: Vec<Span> = Vec::new();

        // Gutter
        if app.show_line_numbers {
            let is_bookmarked = bookmarked_lines.contains(&line_num);
            let gutter_text = match app.line_number_mode {
                LineNumberMode::Absolute => {
                    let prefix = if is_estimated { "~" } else { " " };
                    format!("{}{:>5} ", prefix, line_num + 1)
                }
                LineNumberMode::Relative => {
                    let cursor = pane.cursor_line;
                    let rel = if line_num == cursor {
                        format!("{:>5}", line_num + 1)
                    } else {
                        let diff = line_num.abs_diff(cursor);
                        format!("{:>5}", diff)
                    };
                    format!(" {} ", rel)
                }
                LineNumberMode::LengthBar => {
                    let bar_len = (line_bytes.len() * 5 / app.config.max_line_bytes.max(1)).min(5);
                    let bar: String = std::iter::repeat('█')
                        .take(bar_len)
                        .chain(std::iter::repeat(' ').take(5 - bar_len))
                        .collect();
                    format!(" {} ", bar)
                }
            };

            let gutter_style = if is_bookmarked {
                Style::default().fg(theme.bookmark_fg)
            } else {
                Style::default().fg(theme.gutter_fg)
            };

            let gutter_display = if is_bookmarked {
                let mut g = gutter_text.clone();
                // replace last char before space with bullet
                let len = g.len();
                if len > 1 {
                    g = format!("{}●", &gutter_text[..len - 1]);
                }
                g
            } else {
                gutter_text
            };

            spans.push(Span::styled(gutter_display, gutter_style));
        }

        // Is this the cursor line?
        let is_cursor = line_num == pane.cursor_line;

        // Check visual selection
        let in_visual = if let Some(vs) = app.visual_start {
            let (sel_start, sel_end) = if vs <= pane.cursor_line {
                (vs, pane.cursor_line)
            } else {
                (pane.cursor_line, vs)
            };
            line_num >= sel_start && line_num <= sel_end
        } else {
            false
        };

        // Base line style
        let base_style = if is_cursor {
            Style::default().bg(theme.current_line_bg)
        } else if in_visual {
            Style::default().bg(Color::Rgb(60, 60, 100))
        } else {
            Style::default()
        };

        // Apply format highlighting
        let format_spans = app.file_format.highlight_line(&display_str, theme);

        // Check for search match in this line
        let search_match = search_hits.get(&line_num).copied();

        if format_spans.is_empty() && search_match.is_none() {
            // Fast path: no highlighting
            spans.push(Span::styled(display_str.clone(), base_style));
        } else {
            // Merge format spans and search highlight
            let content_len = display_str.len();
            let mut char_styles: Vec<Style> = vec![base_style; content_len + 1];

            // Apply format spans (byte-indexed into display_str)
            for fs in &format_spans {
                let start = fs.start.min(content_len);
                let end = fs.end.min(content_len);
                for s in char_styles[start..end].iter_mut() {
                    *s = fs.style.patch(base_style);
                    if is_cursor {
                        *s = s.bg(theme.current_line_bg);
                    }
                }
            }

            // Apply search highlight
            if let Some((ms, me)) = search_match {
                let adjusted_start = ms.saturating_sub(h_offset);
                let adjusted_end = me.saturating_sub(h_offset);
                let start = adjusted_start.min(content_len);
                let end = adjusted_end.min(content_len);
                for s in char_styles[start..end].iter_mut() {
                    *s = s.bg(theme.search_highlight_bg).add_modifier(Modifier::BOLD);
                }
            }

            // Build spans from styled chars
            let chars: Vec<char> = display_str.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let style = char_styles[i];
                let start = i;
                while i < chars.len() && char_styles[i] == style {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                spans.push(Span::styled(text, style));
            }
        }

        // Truncation indicator
        if truncated {
            spans.push(Span::styled("›", Style::default().fg(Color::DarkGray)));
        }

        render_lines.push(Line::from(spans));
    }

    let para = Paragraph::new(render_lines);
    f.render_widget(para, area);
}

/// Returns (first_line, last_line) of what's visible in the given pane.
pub fn visible_line_range(app: &App, pane_idx: usize, _area: Rect) -> (u64, u64) {
    let pane = &app.panes[pane_idx];
    let first = pane.scroll_offset;
    let last = first + pane.visible_height as u64;
    (first, last)
}

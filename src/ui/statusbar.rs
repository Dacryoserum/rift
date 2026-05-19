use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::reader::index::{IndexPhase, LinePosition};
use crate::reader::mmap::Encoding;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // ── Left section ─────────────────────────────────────────────────────────
    let file_name = app
        .file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?");

    let encoding_str = match app.reader.encoding {
        Encoding::Utf8 => "UTF-8",
        Encoding::Latin1 => "Latin-1",
        Encoding::Unknown => "Unknown",
    };

    let left = format!(" {} | {} ", file_name, encoding_str);

    // ── Center section ────────────────────────────────────────────────────────
    let center = {
        let index = app.line_index.read().unwrap();
        if index.phase() == IndexPhase::Complete {
            "indexed".to_string()
        } else {
            let pct = if app.reader.file_size > 0 {
                // Estimate progress from ratio of indexed lines * avg line size
                let line_count = index.line_count();
                match index.offset_for_line(line_count.saturating_sub(1)) {
                    LinePosition::Exact { byte_offset, .. }
                    | LinePosition::Estimated { byte_offset, .. } => {
                        (byte_offset * 100 / app.reader.file_size.max(1)) as u8
                    }
                }
            } else {
                0
            };
            let filled = (pct as usize * 8 / 100).min(8);
            let bar: String = std::iter::repeat_n('█', filled)
                .chain(std::iter::repeat_n('░', 8 - filled))
                .collect();
            format!("[{}] {}%", bar, pct)
        }
    };

    // ── Right section ─────────────────────────────────────────────────────────
    let pane = &app.panes[app.active_pane];
    let cursor_line = pane.cursor_line;

    let (line_str, total_str) = {
        let index = app.line_index.read().unwrap();
        let total = index.line_count();
        let pos = index.offset_for_line(cursor_line);
        let is_estimated = matches!(pos, LinePosition::Estimated { .. });
        let line_display = if is_estimated {
            format!("~{}", cursor_line + 1)
        } else {
            format!("{}", cursor_line + 1)
        };
        (line_display, total.to_string())
    };

    let pct = {
        let index = app.line_index.read().unwrap();
        let total = index.line_count().max(1);
        (cursor_line * 100 / total) as u8
    };

    let byte_offset = {
        let index = app.line_index.read().unwrap();
        match index.offset_for_line(cursor_line) {
            LinePosition::Exact { byte_offset, .. }
            | LinePosition::Estimated { byte_offset, .. } => byte_offset,
        }
    };

    let mut right = format!("{}/{} ({}%) | {}B ", line_str, total_str, pct, byte_offset);

    if app.follow_mode {
        right = format!("[FOLLOW] {}", right);
    }

    if let Some(ref q) = app.search_query {
        let n = app.search_results.len();
        right = format!("/{} [{} matches] | {}", q.pattern, n, right);
    }

    // ── Compose ───────────────────────────────────────────────────────────────
    let style = Style::default()
        .fg(theme.statusbar_fg)
        .bg(theme.statusbar_bg);

    // Pad center to fill width
    let left_w = left.chars().count();
    let right_w = right.chars().count();
    let center_w = center.chars().count();
    let total_w = area.width as usize;
    let padding = total_w.saturating_sub(left_w + center_w + right_w);
    let left_pad = padding / 2;
    let right_pad = padding - left_pad;

    let text = format!(
        "{}{}{}{}{}\r",
        left,
        " ".repeat(left_pad),
        center,
        " ".repeat(right_pad),
        right
    );

    let para = Paragraph::new(Line::from(Span::styled(text, style)));
    f.render_widget(para, area);
}

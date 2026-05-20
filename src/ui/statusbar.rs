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

    // Single lock acquisition for all index reads
    let (center, line_str, total_str, pct, byte_offset) = {
        let index = app.line_index.read().unwrap_or_else(|e| e.into_inner());
        let pane = &app.panes[app.active_pane];
        let cursor_line = pane.cursor_line;
        let total = index.line_count();

        let center = if index.phase() == IndexPhase::Complete {
            "indexed".to_string()
        } else {
            let idx_pct = if app.reader.file_size > 0 {
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
            let filled = (idx_pct as usize * 8 / 100).min(8);
            let bar: String = std::iter::repeat_n('█', filled)
                .chain(std::iter::repeat_n('░', 8 - filled))
                .collect();
            format!("[{}] {}%", bar, idx_pct)
        };

        let pos = index.offset_for_line(cursor_line);
        let is_estimated = matches!(pos, LinePosition::Estimated { .. });
        let byte_offset = match pos {
            LinePosition::Exact { byte_offset, .. }
            | LinePosition::Estimated { byte_offset, .. } => byte_offset,
        };
        let line_display = if is_estimated {
            format!("~{}", cursor_line + 1)
        } else {
            format!("{}", cursor_line + 1)
        };
        let pct = (cursor_line * 100 / total.max(1)) as u8;

        (center, line_display, total.to_string(), pct, byte_offset)
    };

    let mut right = format!("{}/{} ({}%) | {}B ", line_str, total_str, pct, byte_offset);

    if app.follow_mode {
        right = format!("[FOLLOW] {}", right);
    }

    if let Some(ref q) = app.search_query {
        let n = app.search_results.len();
        let match_info = match app.search_current {
            Some(i) => format!("[{}/{}]", i + 1, n),
            None if n > 0 => format!("[{} hits]", n),
            _ => String::new(),
        };
        if match_info.is_empty() {
            right = format!("/{} | {}", q.pattern, right);
        } else {
            right = format!("/{} {} | {}", q.pattern, match_info, right);
        }
    }

    // Compose — pad to fill width
    let left_w = left.chars().count();
    let right_w = right.chars().count();
    let center_w = center.chars().count();
    let total_w = area.width as usize;
    let padding = total_w.saturating_sub(left_w + center_w + right_w);
    let left_pad = padding / 2;
    let right_pad = padding - left_pad;

    let text = format!(
        "{}{}{}{}{}",
        left,
        " ".repeat(left_pad),
        center,
        " ".repeat(right_pad),
        right
    );

    let style = Style::default()
        .fg(theme.statusbar_fg)
        .bg(theme.statusbar_bg);

    let para = Paragraph::new(Line::from(Span::styled(text, style)));
    f.render_widget(para, area);
}

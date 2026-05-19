use std::sync::Arc;

use memmap2::Mmap;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::reader::mmap::Encoding;
use crate::ui::popup::centered_popup;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineEnding {
    Lf,
    Crlf,
    Mixed,
    Unknown,
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineEnding::Lf => write!(f, "LF (Unix)"),
            LineEnding::Crlf => write!(f, "CRLF (Windows)"),
            LineEnding::Mixed => write!(f, "Mixed"),
            LineEnding::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileStats {
    pub total_lines: u64,
    pub total_bytes: u64,
    pub min_line_len: usize,
    pub max_line_len: usize,
    pub avg_line_len: f64,
    pub encoding: Encoding,
    pub line_ending: LineEnding,
    pub char_freq: Vec<(char, u64)>,
    pub len_histogram: Vec<u32>,
    pub longest_line_num: u64,
    pub shortest_line_num: u64,
}

/// Compute statistics in a synchronous blocking manner.
pub fn compute_stats(mmap: Arc<Mmap>, file_size: u64) -> FileStats {
    let data: &[u8] = &mmap;

    let mut total_lines: u64 = 0;
    let mut min_line_len: usize = usize::MAX;
    let mut max_line_len: usize = 0;
    let mut total_line_len: u64 = 0;
    let mut longest_line_num: u64 = 0;
    let mut shortest_line_num: u64 = 0;
    let mut lf_count: u64 = 0;
    let mut crlf_count: u64 = 0;

    // Char frequency (limit to ASCII for performance)
    let mut char_freq_map: [u64; 128] = [0; 128];

    // Histogram: 20 buckets (0-10, 10-20, ..., 190-200, 200+)
    let mut len_histogram: Vec<u32> = vec![0u32; 20];

    let mut line_start = 0usize;
    let mut line_num: u64 = 0;

    for (i, &b) in data.iter().enumerate() {
        if b < 128 {
            char_freq_map[b as usize] += 1;
        }

        if b == b'\n' {
            let line_end = if i > 0 && data[i - 1] == b'\r' {
                crlf_count += 1;
                i - 1
            } else {
                lf_count += 1;
                i
            };

            let len = line_end.saturating_sub(line_start);

            if len < min_line_len || total_lines == 0 {
                min_line_len = len;
                shortest_line_num = line_num;
            }
            if len > max_line_len {
                max_line_len = len;
                longest_line_num = line_num;
            }

            total_line_len += len as u64;

            // Histogram bucket
            let bucket = (len / 10).min(19);
            len_histogram[bucket] += 1;

            total_lines += 1;
            line_num += 1;
            line_start = i + 1;
        }
    }

    // Handle last line (no trailing newline)
    if line_start < data.len() {
        let len = data.len() - line_start;
        if len < min_line_len || total_lines == 0 {
            min_line_len = len;
            shortest_line_num = line_num;
        }
        if len > max_line_len {
            max_line_len = len;
            longest_line_num = line_num;
        }
        total_line_len += len as u64;
        let bucket = (len / 10).min(19);
        len_histogram[bucket] += 1;
        total_lines += 1;
    }

    if total_lines == 0 {
        min_line_len = 0;
    }

    let avg_line_len = if total_lines > 0 {
        total_line_len as f64 / total_lines as f64
    } else {
        0.0
    };

    let line_ending = if crlf_count > 0 && lf_count == 0 {
        LineEnding::Crlf
    } else if lf_count > 0 && crlf_count == 0 {
        LineEnding::Lf
    } else if crlf_count > 0 && lf_count > 0 {
        LineEnding::Mixed
    } else {
        LineEnding::Unknown
    };

    // Build top-10 char frequency
    let mut freq_pairs: Vec<(char, u64)> = char_freq_map
        .iter()
        .enumerate()
        .filter(|(i, &c)| *i >= 32 && c > 0) // printable ASCII
        .map(|(i, &c)| (i as u8 as char, c))
        .collect();
    freq_pairs.sort_by(|a, b| b.1.cmp(&a.1));
    freq_pairs.truncate(10);

    // Encoding detection
    let encoding = {
        let sample_len = data.len().min(8192);
        let sample = &data[..sample_len];
        if std::str::from_utf8(sample).is_ok() {
            Encoding::Utf8
        } else {
            Encoding::Latin1
        }
    };

    FileStats {
        total_lines,
        total_bytes: file_size,
        min_line_len,
        max_line_len,
        avg_line_len,
        encoding,
        line_ending,
        char_freq: freq_pairs,
        len_histogram,
        longest_line_num,
        shortest_line_num,
    }
}

pub fn render(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let border_style = Style::default().fg(theme.popup_border_fg);
    let inner = centered_popup(f, "File Statistics", 70, 80, border_style);

    let content_style = Style::default().fg(theme.foreground).bg(theme.popup_bg);

    if app.stats_loading {
        let loading = Paragraph::new("Computing statistics...").style(content_style);
        f.render_widget(loading, inner);
        return;
    }

    let stats = match &app.stats {
        Some(s) => s,
        None => {
            let para =
                Paragraph::new("No statistics available. Press S to compute.").style(content_style);
            f.render_widget(para, inner);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    let label_style = Style::default()
        .fg(theme.json_key_fg)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(theme.foreground);

    macro_rules! stat_line {
        ($label:expr, $value:expr) => {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:20} ", $label), label_style),
                Span::styled($value.to_string(), value_style),
            ]));
        };
    }

    stat_line!("Total lines:", stats.total_lines);
    stat_line!("Total bytes:", format_bytes(stats.total_bytes));
    stat_line!(
        "Encoding:",
        match stats.encoding {
            Encoding::Utf8 => "UTF-8",
            Encoding::Latin1 => "Latin-1",
            Encoding::Unknown => "Unknown",
        }
    );
    stat_line!("Line endings:", stats.line_ending);
    stat_line!("Min line length:", stats.min_line_len);
    stat_line!("Max line length:", stats.max_line_len);
    stat_line!("Avg line length:", format!("{:.1}", stats.avg_line_len));
    stat_line!("Longest line #:", stats.longest_line_num + 1);
    stat_line!("Shortest line #:", stats.shortest_line_num + 1);

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Line Length Distribution:",
        label_style,
    )));

    // Histogram
    let max_bucket = stats
        .len_histogram
        .iter()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    let bar_width = inner.width.saturating_sub(20) as usize;

    for (i, &count) in stats.len_histogram.iter().enumerate() {
        let label = if i < 19 {
            format!("{:3}-{:3}", i * 10, i * 10 + 9)
        } else {
            "200+   ".to_string()
        };
        let filled = if max_bucket > 0 {
            (count as usize * bar_width / max_bucket as usize).min(bar_width)
        } else {
            0
        };
        let bar: String = std::iter::repeat('█').take(filled).collect();
        lines.push(Line::from(vec![
            Span::styled(format!("  {:8} ", label), label_style),
            Span::styled(
                format!("{:<width$} {}", bar, count, width = bar_width),
                value_style,
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Top 10 Characters:",
        label_style,
    )));
    for (ch, freq) in &stats.char_freq {
        let ch_display = if ch.is_control() {
            format!("^{}", (*ch as u8 + 64) as char)
        } else {
            ch.to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {:4} ", ch_display), label_style),
            Span::styled(freq.to_string(), value_style),
        ]));
    }

    let para = Paragraph::new(lines).style(content_style);
    f.render_widget(para, inner);
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB ({} bytes)", bytes as f64 / GB as f64, bytes)
    } else if bytes >= MB {
        format!("{:.2} MB ({} bytes)", bytes as f64 / MB as f64, bytes)
    } else if bytes >= KB {
        format!("{:.2} KB ({} bytes)", bytes as f64 / KB as f64, bytes)
    } else {
        format!("{} bytes", bytes)
    }
}

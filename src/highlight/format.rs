use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone)]
pub enum FileFormat {
    PlainText,
    AnsiLog,
    JsonLines,
    Csv { delimiter: u8, has_header: bool },
    Tsv { has_header: bool },
    Binary,
}

/// A highlighted span within a line: byte range + style
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub style: Style,
}

pub struct FormatDetector;

impl FormatDetector {
    /// Sample first ~8KB (or up to 20 lines) and detect format
    pub fn detect(bytes: &[u8]) -> FileFormat {
        let sample_len = bytes.len().min(8192);
        let sample = &bytes[..sample_len];

        // Binary check: >5% non-printable bytes
        let non_printable = sample
            .iter()
            .filter(|&&b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r')
            .count();
        if non_printable as f64 / sample.len().max(1) as f64 > 0.05 {
            return FileFormat::Binary;
        }

        // Collect up to 20 lines
        let lines: Vec<&[u8]> = sample
            .split(|&b| b == b'\n')
            .take(20)
            .map(|l| strip_cr(l))
            .collect();

        let non_empty_lines: Vec<&[u8]> = lines.iter().copied().filter(|l| !l.is_empty()).collect();

        if non_empty_lines.is_empty() {
            return FileFormat::PlainText;
        }

        // JSON Lines check
        if let Some(first) = non_empty_lines.first() {
            if first.starts_with(b"{") || first.starts_with(b"[") {
                return FileFormat::JsonLines;
            }
        }

        // ANSI Log check
        let log_patterns: &[&[u8]] = &[b"ERROR", b"WARN", b"INFO", b"DEBUG", b"TRACE"];
        for line in non_empty_lines.iter().take(20) {
            // Check for ANSI escape
            if line.windows(2).any(|w| w == b"\x1b[") {
                return FileFormat::AnsiLog;
            }
            // Check for log level keywords
            for pat in log_patterns {
                if contains_bytes(line, pat) {
                    return FileFormat::AnsiLog;
                }
            }
        }

        // CSV check: consistent comma count across first 5 non-empty lines
        if non_empty_lines.len() >= 2 {
            let comma_counts: Vec<usize> = non_empty_lines
                .iter()
                .take(5)
                .map(|l| l.iter().filter(|&&b| b == b',').count())
                .collect();
            if comma_counts.iter().all(|&c| c >= 2) {
                let first = comma_counts[0];
                let consistent = comma_counts.iter().all(|&c| c == first || c.abs_diff(first) <= 1);
                if consistent {
                    return FileFormat::Csv {
                        delimiter: b',',
                        has_header: true,
                    };
                }
            }

            // TSV check
            let tab_counts: Vec<usize> = non_empty_lines
                .iter()
                .take(5)
                .map(|l| l.iter().filter(|&&b| b == b'\t').count())
                .collect();
            if tab_counts.iter().all(|&c| c >= 1) {
                let first = tab_counts[0];
                let consistent = tab_counts.iter().all(|&c| c == first || c.abs_diff(first) <= 1);
                if consistent {
                    return FileFormat::Tsv { has_header: true };
                }
            }
        }

        FileFormat::PlainText
    }
}

fn strip_cr(line: &[u8]) -> &[u8] {
    if line.last() == Some(&b'\r') {
        &line[..line.len() - 1]
    } else {
        line
    }
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle)
}

impl FileFormat {
    /// Return highlight spans for a single decoded line.
    pub fn highlight_line(&self, line: &str, theme: &crate::config::Theme) -> Vec<HighlightSpan> {
        match self {
            FileFormat::AnsiLog => highlight_log(line, theme),
            FileFormat::JsonLines => highlight_json(line, theme),
            FileFormat::Csv { delimiter, has_header: _ } => {
                highlight_csv(line, *delimiter, theme)
            }
            FileFormat::Tsv { has_header: _ } => highlight_csv(line, b'\t', theme),
            FileFormat::PlainText | FileFormat::Binary => vec![],
        }
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, FileFormat::Binary)
    }
}

fn highlight_log(line: &str, theme: &crate::config::Theme) -> Vec<HighlightSpan> {
    let (color, found) = if line.contains("ERROR") {
        (theme.log_error_fg, true)
    } else if line.contains("WARN") {
        (theme.log_warn_fg, true)
    } else if line.contains("INFO") {
        (theme.log_info_fg, true)
    } else if line.contains("DEBUG") {
        (theme.log_debug_fg, true)
    } else if line.contains("TRACE") {
        (theme.log_debug_fg, true)
    } else {
        (theme.foreground, false)
    };

    if found {
        vec![HighlightSpan {
            start: 0,
            end: line.len(),
            style: Style::default().fg(color),
        }]
    } else {
        vec![]
    }
}

fn highlight_json(line: &str, theme: &crate::config::Theme) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    #[derive(PartialEq, Clone, Copy)]
    enum State {
        Normal,
        InString,
        AfterKey,
    }

    let mut state = State::Normal;
    let mut string_start = 0usize;

    while i < len {
        match state {
            State::Normal => {
                if bytes[i] == b'"' {
                    string_start = i;
                    state = State::InString;
                } else if bytes[i].is_ascii_digit() || bytes[i] == b'-' {
                    // Number
                    let start = i;
                    while i < len
                        && (bytes[i].is_ascii_digit()
                            || bytes[i] == b'.'
                            || bytes[i] == b'e'
                            || bytes[i] == b'E'
                            || bytes[i] == b'-'
                            || bytes[i] == b'+')
                    {
                        i += 1;
                    }
                    spans.push(HighlightSpan {
                        start,
                        end: i,
                        style: Style::default().fg(theme.json_number_fg),
                    });
                    continue;
                } else if bytes[i..].starts_with(b"true")
                    || bytes[i..].starts_with(b"false")
                    || bytes[i..].starts_with(b"null")
                {
                    let kw_len = if bytes[i..].starts_with(b"null") {
                        4
                    } else if bytes[i..].starts_with(b"true") {
                        4
                    } else {
                        5
                    };
                    spans.push(HighlightSpan {
                        start: i,
                        end: i + kw_len,
                        style: Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    });
                    i += kw_len;
                    continue;
                }
            }
            State::InString => {
                if bytes[i] == b'\\' {
                    i += 2; // skip escaped character
                    continue;
                }
                if bytes[i] == b'"' {
                    let end = i + 1;
                    let mut j = end;
                    while j < len && bytes[j] == b' ' {
                        j += 1;
                    }
                    let is_key = j < len && bytes[j] == b':';
                    let style = if is_key {
                        Style::default().fg(theme.json_key_fg).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.json_string_fg)
                    };
                    spans.push(HighlightSpan {
                        start: string_start,
                        end,
                        style,
                    });
                    state = if is_key { State::AfterKey } else { State::Normal };
                }
            }
            State::AfterKey => {
                // Skip colon
                if bytes[i] == b':' {
                    state = State::Normal;
                }
            }
        }
        i += 1;
    }

    spans
}

fn highlight_csv(line: &str, delimiter: u8, theme: &crate::config::Theme) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let mut col = 0usize;
    let mut col_start = 0usize;
    let bytes = line.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        if b == delimiter {
            let color = if col % 2 == 0 {
                theme.csv_odd_col_fg
            } else {
                theme.csv_even_col_fg
            };
            spans.push(HighlightSpan {
                start: col_start,
                end: i,
                style: Style::default().fg(color),
            });
            col += 1;
            col_start = i + 1;
        }
    }

    // Last column
    if col_start <= line.len() {
        let color = if col % 2 == 0 {
            theme.csv_odd_col_fg
        } else {
            theme.csv_even_col_fg
        };
        spans.push(HighlightSpan {
            start: col_start,
            end: line.len(),
            style: Style::default().fg(color),
        });
    }

    spans
}

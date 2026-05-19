use std::path::PathBuf;
use std::process::Command;

pub trait ClipboardBackend: Send {
    fn set_text(&mut self, text: &str) -> anyhow::Result<()>;
    fn name(&self) -> &'static str;
}

// ── ArboardClipboard ─────────────────────────────────────────────────────────

#[cfg(feature = "clipboard")]
struct ArboardClipboard {
    ctx: arboard::Clipboard,
}

#[cfg(feature = "clipboard")]
impl ClipboardBackend for ArboardClipboard {
    fn set_text(&mut self, text: &str) -> anyhow::Result<()> {
        self.ctx.set_text(text)?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "arboard"
    }
}

// ── SubprocessClipboard ───────────────────────────────────────────────────────

struct SubprocessClipboard {
    program: &'static str,
    args: &'static [&'static str],
}

impl ClipboardBackend for SubprocessClipboard {
    fn set_text(&mut self, text: &str) -> anyhow::Result<()> {
        use std::io::Write;
        let mut child = Command::new(self.program)
            .args(self.args)
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "subprocess"
    }
}

// ── FileClipboard ─────────────────────────────────────────────────────────────

struct FileClipboard {
    path: PathBuf,
}

impl ClipboardBackend for FileClipboard {
    fn set_text(&mut self, text: &str) -> anyhow::Result<()> {
        std::fs::write(&self.path, text)?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "file"
    }
}

// ── Detection ─────────────────────────────────────────────────────────────────

/// Detect best available clipboard backend at startup.
pub fn detect_clipboard() -> Box<dyn ClipboardBackend> {
    // 1. Try arboard (feature-gated)
    #[cfg(feature = "clipboard")]
    {
        if let Ok(ctx) = arboard::Clipboard::new() {
            return Box::new(ArboardClipboard { ctx });
        }
    }

    // 2. Try subprocess
    if let Some(backend) = detect_subprocess() {
        return backend;
    }

    // 3. Fallback: file
    let path = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".rift_yank");

    Box::new(FileClipboard { path })
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn detect_subprocess() -> Option<Box<dyn ClipboardBackend>> {
    // macOS
    if command_exists("pbcopy") {
        return Some(Box::new(SubprocessClipboard {
            program: "pbcopy",
            args: &[],
        }));
    }

    // Linux: xclip
    if command_exists("xclip") {
        return Some(Box::new(SubprocessClipboard {
            program: "xclip",
            args: &["-selection", "clipboard"],
        }));
    }

    // Linux: xsel
    if command_exists("xsel") {
        return Some(Box::new(SubprocessClipboard {
            program: "xsel",
            args: &["--clipboard", "--input"],
        }));
    }

    // Wayland: wl-copy
    if command_exists("wl-copy") {
        return Some(Box::new(SubprocessClipboard {
            program: "wl-copy",
            args: &[],
        }));
    }

    None
}

# rift

Navigate the depths of massive text files.

`rift` is a terminal UI viewer built for files that are too large for ordinary tools — logs, datasets, exports, anything from a few hundred megabytes to tens of gigabytes. It opens instantly via memory-mapped IO, starts navigation before the line index is complete, and provides features well beyond `less`.

```
┌─────────────────────────────────────────────────┬───┐
│   1  2026-05-19 ERROR connection refused         │   │
│   2  2026-05-19 INFO  retry attempt 1            │▐  │
│   3  2026-05-19 WARN  timeout after 30s          │▐  │
│   4  2026-05-19 ERROR upstream unreachable       │●  │
│   5  2026-05-19 INFO  fallback activated         │   │
│ ●~6  2026-05-19 DEBUG handler registered         │   │
│   7  2026-05-19 INFO  request processed          │   │
├─────────────────────────────────────────────────┴───┤
│ server.log │ UTF-8 │ LF │ ████████░░ 82% │ 7/~1.2M  │
│ / error                                              │
└──────────────────────────────────────────────────────┘
```

## Features

**Navigation**
- Vim keybindings: `j/k`, `Ctrl+U/D`, `Ctrl+B/F`, `g/G`
- Jump anywhere: `:{n}` (line), `{n}%` (percentage), `:{n}b` (byte offset)
- Navigation history: `Ctrl+O` / `Ctrl+I` (jumplist, like vim)
- Split pane: `Ctrl+W` — two independent positions in the same file

**Search**
- `/` regex forward, `?` backward, `n/N` next/prev
- All matches highlighted simultaneously in the viewport
- `F` — fuzzy line search popup (fzf-style)
- Background search thread — stream results while navigating

**Visual**
- Minimap sidebar with braille density encoding — see the whole file structure at a glance
- Format-aware highlighting: log levels (ERROR/WARN/INFO), JSON Lines, CSV/TSV
- Three gutter modes: `l` absolute numbers, `L` relative, `~` line-length bar
- Line wrap toggle: `w`

**Bookmarks**
- `m{a-z}` — set a named bookmark, `'{a-z}` — jump to it
- `B` — bookmark manager (list, delete, jump)
- Bookmarks persist across sessions per file

**Analysis**
- `S` — statistics panel: line length histogram, character frequency, encoding, line ending style
- Encoding detection: UTF-8 / Latin-1 displayed in status bar

**Follow mode**
- `f` — tail the file live as it grows (like `tail -f`)

**Copy / Export**
- `y` — yank current line to clipboard
- `V` + motion + `Y` — yank a line range
- `:export {start},{end} {path}` — write a range of lines to a file

## Installation

### Pre-built binary (macOS arm64)

```sh
curl -L https://github.com/Dacryoserum/rift/releases/latest/download/rift-v0.1.1-macos-arm64.tar.gz | tar -xz
install -m755 rift ~/.local/bin/rift
```

### From source

```sh
git clone https://github.com/Dacryoserum/rift
cd rift
make install
```

### Requirements

- Rust 1.75+

## Usage

```sh
rift <file>
rift server.log
rift --line 50000 huge.csv     # open at line 50,000
rift --byte 1048576 dump.txt   # open at byte offset 1MB
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Ctrl+D` | Half page down |
| `Ctrl+U` | Half page up |
| `Ctrl+F` / `PgDn` | Full page down |
| `Ctrl+B` / `PgUp` | Full page up |
| `g g` | Go to first line |
| `G` | Go to last line |
| `:{n}` | Jump to line n |
| `{n}%` | Jump to n% through file |
| `:{n}b` | Jump to byte offset n |
| `Ctrl+O` | Jump back in history |
| `Ctrl+I` | Jump forward in history |
| `/` | Search forward (regex) |
| `?` | Search backward (regex) |
| `n` | Next match |
| `N` | Previous match |
| `F` | Fuzzy line search |
| `l` | Toggle line numbers |
| `L` | Cycle gutter mode (absolute / relative / length-bar) |
| `~` | Line-length bar gutter |
| `w` | Toggle line wrap |
| `m{a-z}` | Set bookmark |
| `'{a-z}` | Jump to bookmark |
| `B` | Bookmark manager |
| `S` | Statistics panel |
| `f` | Toggle follow mode |
| `y` | Yank current line |
| `V` | Start visual line selection |
| `Y` | Yank selection (in visual mode) |
| `Ctrl+W` | Toggle split pane |
| `Tab` | Switch active pane |
| `q` | Quit |

### Command line (`:`)

| Command | Action |
|---------|--------|
| `:{n}` | Jump to line n |
| `{n}%` | Jump to n% |
| `:{n}b` | Jump to byte offset |
| `:export {s},{e} {path}` | Export lines s–e to file |
| `:q` / `:quit` | Quit |
| `:split` | Toggle split pane |

## Configuration

`~/.config/rift/config.toml`:

```toml
tab_size = 4
follow_poll_interval_ms = 250
minimap_enabled = true
theme = "Dark"   # "Dark" | "Light" | "Solarized"
```

## How it works

`rift` memory-maps the file — the OS handles paging, so the file is never fully loaded into RAM. A background thread builds a line offset index in two phases: a fast sampling pass (completes in ~0.5s for a 5GB file on NVMe), then a full sequential scan. Navigation works immediately using estimated positions from the sampling pass, and becomes exact as the full scan completes.

Search runs on a separate cancellable thread and streams results back to the UI. Bookmarks are persisted per-file using the file's inode + size + mtime as a key, so they survive renames as long as the content is unchanged.

## License

Licensed under either of MIT or Apache-2.0 at your option.

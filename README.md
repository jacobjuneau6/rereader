# rereader

A terminal-based ebook reader with automatic progress saving. Read EPUB and MOBI files, pick up right where you left off.

## Features

- **EPUB & MOBI support** — auto-detects format; no conversion needed
- **Persistent progress** — saves your chapter and scroll position on exit, restores it on next open
- **Vim-like keybindings** — `j`/`k` to scroll, `g`/`G` to jump, `n`/`p` for chapters
- **Clean TUI** — built with [ratatui](https://ratatui.rs), showing book title, chapter, and progress

## Compatibility

| Platform | Status |
|----------|--------|
| Linux    | ✅ Supported |
| macOS    | ❌ Not yet |
| Windows  | ❌ Not yet |

## Install

```bash
cargo install rereader
```

## Usage

```bash
rereader path/to/book.epub
```

### Keybindings

| Key | Action |
|-----|--------|
| `q` / `Esc` | Quit (auto-saves position) |
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `d` / `PgDn` | Page down |
| `u` / `PgUp` | Page up |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `n` / `→` | Next chapter |
| `p` / `←` | Previous chapter |

## How it works

Progress is stored in `~/.config/rereader/progress.json`, keyed by the canonical file path. Each entry records the chapter index and line-offset. The file is written on every normal exit (`q` or `Esc`).

For MOBI files, chapters are split at `<h1>`–`<h3>` heading tags. EPUBs use the spine-defined chapter structure.

## License

MIT
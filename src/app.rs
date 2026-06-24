use std::any;

use crate::reader::{Chapter, Reader};
use crate::state::StateManager;

pub struct App {
    pub reader: Reader,
    state_mgr: StateManager,
    epub_path: std::path::PathBuf,
    pub scroll: usize,
}

impl App {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let epub_path = std::path::PathBuf::from(path)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(path));
        let reader = Reader::open(&epub_path)?;
        let state_mgr = StateManager::new();

        let (chapter_idx, scroll) = state_mgr
            .load(&epub_path)
            .and_then(|entry| {
                if entry.chapter_index < reader.chapter_count() {
                    Some((entry.chapter_index, entry.scroll_offset))
                } else {
                    None
                }
            })
            .unwrap_or((0,0));
        let mut app = Self {
            reader,
            state_mgr,
            epub_path,
            scroll: 0,
        };

        app.reader.set_chapter(chapter_idx);
        app.scroll = app
            .current_chapter()
            .map(|c| {
                let total = c.wrapped_lines(80).len().saturating_sub(1);
                scroll.min(total)
            })
            .unwrap_or(0);
        Ok(app)
    }

    pub fn save_state(&mut self) -> anyhow::Result<()> {
        self.state_mgr.save(
            &self.epub_path,
            self.reader.current_chapter_index(),
            self.scroll,
            &self.reader.title,
        )
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return true,

            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(1),
            KeyCode::Char('d') => self.scroll_down(10),
            KeyCode::Char('u') => self.scroll_up(10),
            KeyCode::PageDown => self.page_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::Char('g') => self.scroll_to_top(),
            KeyCode::Char('G') => self.scroll_to_bottom(),
            KeyCode::Home => self.scroll_to_top(),
            KeyCode::End => self.scroll_to_bottom(),
            KeyCode::Char('n') | KeyCode::Right => self.next_chapter(),
            KeyCode::Char('p') | KeyCode::Left => self.prev_chapter(),
            KeyCode::Char('N') => self.next_chapter(),
            KeyCode::Char('P') => self.prev_chapter(),
            _ => {}
        }
        false
    }

    fn max_scroll(&self) -> usize {
        self.current_chapter()
            .map(|c| {
                let len = c.wrapped_lines(80).len();
                len.saturating_sub(1)
            })
            .unwrap_or(0)
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = (self.scroll + n).min(self.max_scroll());
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.max_scroll();
    }

    pub fn page_up(&mut self) {
        self.scroll_up(20);
    }

    pub fn page_down(&mut self) {
        self.scroll_down(20);
    }

    // ── chapter navigation ──────────────────────────────────────────

    fn next_chapter(&mut self) {
        if self.reader.next_chapter() {
            self.scroll = 0;
        }
    }

    fn prev_chapter(&mut self) {
        if self.reader.prev_chapter() {
            self.scroll = self.max_scroll();
        }
    }

    // ── accessors for the UI ────────────────────────────────────────

    pub fn current_chapter(&self) -> Option<&Chapter> {
        self.reader.current_chapter()
    }

    pub fn chapter_index(&self) -> usize {
        self.reader.current_chapter_index()
    }

    pub fn chapter_count(&self) -> usize {
        self.reader.chapter_count()
    }

    pub fn title(&self) -> &str {
        &self.reader.title
    }

    pub fn author(&self) -> &str {
        &self.reader.author
    }

    /// Overall progress through the book, 0.0–1.0.
    pub fn progress(&self) -> f64 {
        let total_chaps = self.chapter_count().max(1);
        let chap_progress = if let Some(ch) = self.current_chapter() {
            let lines = ch.wrapped_lines(80);
            let max = lines.len().saturating_sub(1).max(1);
            self.scroll as f64 / max as f64
        } else {
            0.0
        };
        (self.chapter_index() as f64 + chap_progress) / total_chaps as f64
    }
}
use anyhow::Context;
use scraper::{Html, Selector};
use std::path::Path;

/// A single chapter / section extracted from the ebook.
pub struct Chapter {
    pub title: String,
    /// Clean plain-text content.
    pub text: String,
    /// Cached word-wrapped lines for a specific terminal width.
    cached_lines: std::cell::RefCell<Option<(usize, Vec<String>)>>,
}

impl Chapter {
    /// Return `text` split into lines wrapped to `width` columns.
    /// The result is cached so we don't re-wrap on every frame.
    pub fn wrapped_lines(&self, width: usize) -> std::rc::Rc<[String]> {
        let mut cache = self.cached_lines.borrow_mut();
        if let Some((w, lines)) = cache.as_ref()
            && *w == width
        {
            return std::rc::Rc::from(lines.as_slice());
        }
        let lines = wrap_text(&self.text, width);
        let rc: std::rc::Rc<[String]> = lines.into();
        *cache = Some((width, rc.to_vec()));
        rc
    }
}

/// Holds parsed chapter content and tracks the current reading position.
pub struct Reader {
    pub title: String,
    pub author: String,
    chapters: Vec<Chapter>,
    chapter_idx: usize,
}

impl Reader {
    /// Open an ebook file.  Tries EPUB first, then falls back to MOBI.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        // Try EPUB first — it's a ZIP-based format.
        if let Ok(reader) = try_epub(path) {
            return Ok(reader);
        }
        // Fall back to MOBI (Kindle / Mobipocket).
        try_mobi(path).context("file is not a readable EPUB or MOBI ebook")
    }

    pub fn chapter_count(&self) -> usize {
        self.chapters.len()
    }

    pub fn current_chapter_index(&self) -> usize {
        self.chapter_idx
    }

    pub fn current_chapter(&self) -> Option<&Chapter> {
        self.chapters.get(self.chapter_idx)
    }

    pub fn set_chapter(&mut self, idx: usize) {
        if idx < self.chapters.len() {
            self.chapter_idx = idx;
        }
    }

    pub fn next_chapter(&mut self) -> bool {
        if self.chapter_idx + 1 < self.chapters.len() {
            self.chapter_idx += 1;
            true
        } else {
            false
        }
    }

    pub fn prev_chapter(&mut self) -> bool {
        if self.chapter_idx > 0 {
            self.chapter_idx -= 1;
            true
        } else {
            false
        }
    }
}

// ── EPUB loading ────────────────────────────────────────────────────────

fn try_epub(path: &Path) -> anyhow::Result<Reader> {
    let mut doc =
        epub::doc::EpubDoc::new(path).context("failed to open EPUB file")?;

    let title = doc.get_title().unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    });

    let author = doc
        .mdata("creator")
        .map(|item| item.value.clone())
        .unwrap_or_default();

    // Collect spine IDs so we don't hold an immutable borrow during &mut calls.
    let spine_ids: Vec<String> =
        doc.spine.iter().map(|item| item.idref.clone()).collect();

    let mut chapters = Vec::with_capacity(spine_ids.len());

    for id in &spine_ids {
        let (html_str, _mime) = match doc.get_resource_str(id) {
            Some(s) => s,
            None => continue,
        };
        if html_str.trim().is_empty() {
            continue;
        }
        let (chap_title, text) = extract_text(&html_str);
        let title = fallback_title(chap_title, &text);
        chapters.push(Chapter {
            title,
            text,
            cached_lines: std::cell::RefCell::new(None),
        });
    }

    anyhow::ensure!(
        !chapters.is_empty(),
        "EPUB has no readable content chapters"
    );

    Ok(Reader {
        title,
        author,
        chapters,
        chapter_idx: 0,
    })
}

// ── MOBI loading ────────────────────────────────────────────────────────

fn try_mobi(path: &Path) -> anyhow::Result<Reader> {
    let mobi = mobi::Mobi::from_path(path).context("failed to open MOBI file")?;

    let title = mobi.title();
    let author = mobi.author().unwrap_or_default();

    // MOBI gives us one big HTML document.
    let html_str = mobi.content_as_string_lossy();

    let chapters = split_mobi_into_chapters(&html_str);

    anyhow::ensure!(
        !chapters.is_empty(),
        "MOBI file has no readable content"
    );

    Ok(Reader {
        title,
        author,
        chapters,
        chapter_idx: 0,
    })
}

/// Split a MOBI HTML document into pseudo-chapters using heading tags.
fn split_mobi_into_chapters(html: &str) -> Vec<Chapter> {
    let document = Html::parse_document(html);

    // Try to find the <body>; fall back to the whole document.
    let body_sel = Selector::parse("body").ok();
    let root = body_sel
        .as_ref()
        .and_then(|s| document.select(s).next());

    let mut chapters: Vec<Chapter> = Vec::new();
    let mut current_title = String::new();
    let mut current_html = String::new();

    let children: Vec<_> = match root {
        Some(body) => body.children().collect(),
        None => document.root_element().children().collect(),
    };

    for child in &children {
        match child.value() {
            scraper::node::Node::Element(el) => {
                let tag = el.name();
                let is_heading = matches!(tag, "h1" | "h2" | "h3");

                if is_heading {
                    // Flush the previous chapter.
                    if !current_html.trim().is_empty() || !current_title.is_empty() {
                        let (_, text) = extract_text(&current_html);
                        let title = fallback_title(
                            std::mem::take(&mut current_title),
                            &text,
                        );
                        if !text.trim().is_empty() {
                            chapters.push(Chapter {
                                title,
                                text,
                                cached_lines: std::cell::RefCell::new(None),
                            });
                        }
                        current_html.clear();
                    }

                    // Start new chapter heading.
                    if let Some(el_ref) = scraper::ElementRef::wrap(*child) {
                        current_title = el_ref
                            .text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ");
                    }
                }

                // Serialize the element back to HTML-like text for extract_text.
                if let Some(el_ref) = scraper::ElementRef::wrap(*child) {
                    current_html.push_str(&el_ref.html());
                }
            }
            scraper::node::Node::Text(text) => {
                current_html.push_str(text);
            }
            _ => {}
        }
    }

    // Flush the final chapter.
    if !current_html.trim().is_empty() || !current_title.is_empty() {
        let (_, text) = extract_text(&current_html);
        let title = fallback_title(current_title, &text);
        if !text.trim().is_empty() {
            chapters.push(Chapter {
                title,
                text,
                cached_lines: std::cell::RefCell::new(None),
            });
        }
    }

    // If splitting produced nothing useful, use the whole document as one chapter.
    if chapters.is_empty() {
        let (title, text) = extract_text(html);
        let title = fallback_title(title, &text);
        if !text.trim().is_empty() {
            chapters.push(Chapter {
                title,
                text,
                cached_lines: std::cell::RefCell::new(None),
            });
        }
    }

    chapters
}

fn fallback_title(heading: String, text: &str) -> String {
    if heading.is_empty() {
        let preview: String = text.chars().take(60).collect();
        preview.trim().to_string()
    } else {
        heading
    }
}

// ── HTML → text extraction ───────────────────────────────────────────

/// Return `(heading_text, body_text)` from XHTML / HTML.
fn extract_text(html: &str) -> (String, String) {
    let document = Html::parse_document(html);

    // Grab the first heading element for the chapter title.
    let h_sel = Selector::parse("h1, h2, h3, h4, h5, h6, title").ok();
    let mut heading = String::new();

    if let Some(ref sel) = h_sel
        && let Some(el) = document.select(sel).next()
    {
        heading = el
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    }

    // Walk <body> (or the whole document) collecting block-aware text.
    let body_sel = Selector::parse("body").ok();
    let body_text = match body_sel.as_ref().and_then(|s| document.select(s).next()) {
        Some(body) => collect_block_text(&body),
        None => {
            let mut text = String::new();
            for node in document.root_element().descendants() {
                if let Some(t) = node.value().as_text() {
                    text.push_str(t);
                }
            }
            collapse_blanks(&text)
        }
    };

    (heading, body_text)
}

/// Walk an element tree and emit text, inserting newlines at block boundaries.
fn collect_block_text(element: &scraper::ElementRef) -> String {
    const BLOCK_TAGS: &[&str] = &[
        "p", "div", "br", "h1", "h2", "h3", "h4", "h5", "h6",
        "li", "tr", "hr", "section", "article", "header", "footer",
        "blockquote", "pre", "table",
    ];

    let mut output = String::new();
    collect_text(element, BLOCK_TAGS, &mut output);
    collapse_blanks(&output)
}

fn collect_text(
    element: &scraper::ElementRef,
    block_tags: &[&str],
    out: &mut String,
) {
    for child in element.children() {
        match child.value() {
            scraper::node::Node::Text(text) => {
                out.push_str(text);
            }
            scraper::node::Node::Element(el) => {
                let tag = el.name();
                let is_block = block_tags.contains(&tag);

                if is_block && !out.ends_with('\n') && !out.is_empty() {
                    out.push('\n');
                }

                let el_ref = scraper::ElementRef::wrap(child).unwrap();

                if tag == "br" {
                    out.push('\n');
                    continue;
                }
                if tag == "hr" {
                    if !out.ends_with('\n') && !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str("───\n");
                    continue;
                }

                collect_text(&el_ref, block_tags, out);

                if is_block && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            _ => {}
        }
    }
}

/// Collapse consecutive blank lines and trim leading/trailing whitespace.
fn collapse_blanks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut blank_count = 0usize;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }
    result.trim().to_string()
}

// ── Word wrapping ─────────────────────────────────────────────────────

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        lines.extend(textwrap::wrap_paragraph(paragraph, width));
    }
    lines
}

mod textwrap {
    pub(super) fn wrap_paragraph(text: &str, width: usize) -> Vec<String> {
        let width = width.max(1);
        let mut lines: Vec<String> = Vec::new();
        for word in text.split_whitespace() {
            if let Some(last) = lines.last_mut() {
                let candidate = format!("{last} {word}");
                if display_width(&candidate) <= width {
                    *last = candidate;
                    continue;
                }
            }
            // Start new line.  If the word itself exceeds `width`, hard-break.
            if display_width(word) <= width {
                lines.push(word.to_string());
            } else {
                lines.extend(hard_break(word, width));
            }
        }
        lines
    }

    fn display_width(s: &str) -> usize {
        unicode_width::UnicodeWidthStr::width(s)
    }

    fn hard_break(word: &str, width: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut current_w = 0usize;
        for ch in word.chars() {
            let ch_w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_w + ch_w > width && !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
                current_w = 0;
            }
            current.push(ch);
            current_w += ch_w;
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        chunks
    }
}

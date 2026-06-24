use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Fill(1),   // body
        Constraint::Length(1), // footer
    ])
    .split(area);

    draw_header(frame, chunks[0], app);
    draw_body(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);
}

// ── header ─────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let author_str = if app.author().is_empty() {
        String::new()
    } else {
        format!(" — {}", app.author())
    };

    let current = app.current_chapter();
    let chap_title = current.map(|c| c.title.as_str()).unwrap_or("");

    let header_text = vec![
        Line::from(vec![
            Span::styled(
                app.title(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(author_str, Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled(
                format!(
                    "Ch {}/{}: ",
                    app.chapter_index() + 1,
                    app.chapter_count()
                ),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(chap_title, Style::default().fg(Color::White)),
            Span::styled(
                format!("  [{:.0}%]", app.progress() * 100.0),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(Text::from(header_text))
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

// ── body ───────────────────────────────────────────────────────────

fn draw_body(frame: &mut Frame, area: Rect, app: &App) {
    // Small margin so text doesn't hug the edge.
    let inner = Rect {
        x: area.x + 1,
        width: area.width.saturating_sub(2).max(1),
        ..area
    };

    let width = inner.width as usize;

    let lines: Vec<Line> = if let Some(chap) = app.current_chapter() {
        let wrapped = chap.wrapped_lines(width);
        let total = wrapped.len();
        let start = app.scroll.min(total.saturating_sub(1));

        wrapped[start..]
            .iter()
            .map(|s| {
                if s.is_empty() {
                    Line::from("")
                } else {
                    // Clone so the Line owns its text — we drop the Rc below.
                    Line::from(Span::styled(
                        s.clone(),
                        Style::default().fg(Color::White),
                    ))
                }
            })
            .collect()
    } else {
        vec![Line::from(Span::styled(
            "No content",
            Style::default().fg(Color::Red),
        ))]
    };

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

// ── footer ─────────────────────────────────────────────────────────

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let progress = app.progress() * 100.0;
    let chap_info = format!(
        "Chapter {}/{}  ·  {:.0}%",
        app.chapter_index() + 1,
        app.chapter_count(),
        progress
    );
    let scroll_info = format!("scroll: {}", app.scroll);

    let help = format!(
        "{}  │  {}  │  q quit  j/k up/down  n/p next/prev  g/G top/bottom  d/u pgdn/up",
        chap_info, scroll_info
    );

    let line = Line::from(Span::styled(
        help,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    ));

    let paragraph = Paragraph::new(Text::from(line))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

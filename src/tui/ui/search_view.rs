use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style}, text::{Line, Span}, widgets::{Block, BorderType, Borders, List, ListItem, Paragraph}};
use crate::tui::app::App;
use super::{ACCENT, GREEN, block, status};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(app.search_query.as_str())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(" 🔍 Search ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))),
        chunks[0],
    );

    let title = format!("Results: {}", app.search_results.len());
    let items: Vec<ListItem> = app.search_results.iter().map(|r|
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:>6} ", r.score as i64), Style::default().fg(GREEN)),
            Span::styled(r.command.clone(), Style::default()),
        ]))
    ).collect();

    f.render_stateful_widget(
        List::new(items).block(block(&title))
            .highlight_style(Style::default().bg(ratatui::style::Color::Rgb(40,44,52)).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        chunks[1], &mut app.search_state.clone(),
    );
    f.render_widget(status("Type to search  •  ↑↓ navigate  •  Esc back"), chunks[2]);
}

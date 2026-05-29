use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, BorderType, Borders, List, ListItem, Paragraph}};
use crate::tui::app::App;
use super::{ACCENT, DIM, GREEN, block, status};

pub fn render(f: &mut Frame, app: &App, area: Rect, title: &str) {
    let [list, bar] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)]).split(area)[..] else { return };

    let items: Vec<ListItem> = app.output_lines.iter().map(|line| {
        if line.is_empty() { return ListItem::new(Line::from("")); }
        if line.starts_with("──") {
            return ListItem::new(Line::from(Span::styled(line.as_str(), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))));
        }
        if let Some((label, val)) = line.split_once(':') {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<25}:", label), Style::default().fg(DIM)),
                Span::styled(val.to_string(), Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
            ]))
        } else {
            ListItem::new(Line::from(Span::styled(line.as_str(), Style::default().fg(Color::White))))
        }
    }).collect();

    f.render_widget(List::new(items).block(block(title)), list);
    f.render_widget(status("Esc / q to go back"), bar);
}

pub fn render_query(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new("SELECT * COMMANDS WHERE frequency > 5 ORDER BY score LIMIT 10")
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
            .style(Style::default().fg(DIM)),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(app.query_input.as_str())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(" Query ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))),
        chunks[1],
    );

    let items: Vec<ListItem> = app.output_lines.iter()
        .map(|l| ListItem::new(Span::raw(l.as_str()))).collect();
    f.render_widget(List::new(items).block(block("Results")), chunks[2]);
    f.render_widget(status("Type query  •  Enter run  •  Esc back"), chunks[3]);
}

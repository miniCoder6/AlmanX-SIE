use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style}, text::{Line, Span}, widgets::{Block, BorderType, Borders, List, ListItem, Paragraph}};
use crate::tui::app::App;
use super::{ACCENT, DIM, block, status};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(app.input.as_str())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(" Command ", Style::default().fg(ACCENT)))),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(app.alias_input.as_str())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(" Alias  (Tab → suggestions) ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))),
        chunks[1],
    );

    let items: Vec<ListItem> = app.suggestions.iter().map(|s|
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<10}", s.alias), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {}", s.reason), Style::default().fg(DIM)),
        ]))
    ).collect();
    f.render_widget(List::new(items).block(block("Suggestions")), chunks[2]);
    f.render_widget(status("Type alias  •  Tab pick  •  Enter confirm  •  Esc back"), chunks[3]);
}

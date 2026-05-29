use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style}, text::{Line, Span}, widgets::{List, ListItem}};
use crate::tui::app::App;
use super::{ACCENT, DIM, block, status};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let [list, bar] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)]).split(area)[..] else { return };

    let items: Vec<ListItem> = app.suggestions.iter().map(|s|
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<12}", s.alias), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", s.reason), Style::default().fg(DIM)),
        ]))
    ).collect();
    f.render_stateful_widget(
        List::new(items).block(block("Pick Suggestion"))
            .highlight_style(Style::default().bg(ratatui::style::Color::Rgb(40,44,52)).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        list, &mut app.sug_state.clone(),
    );
    f.render_widget(status("↑↓ navigate  •  Enter select  •  Esc back"), bar);
}

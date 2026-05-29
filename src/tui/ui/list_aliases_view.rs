use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style}, text::{Line, Span}, widgets::{List, ListItem}};
use crate::tui::app::App;
use super::{ACCENT, block, status};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let [list, bar] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)]).split(area)[..] else { return };

    let wa = app.aliases.iter().map(|(a,_)| a.len()).max().unwrap_or(5).max(5);
    let items: Vec<ListItem> = app.aliases.iter().map(|(a, c)|
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<w$}", a, w=wa+2), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw("= "),
            Span::raw(c.clone()),
        ]))
    ).collect();

    f.render_stateful_widget(
        List::new(items).block(block(&format!("Aliases ({})", app.aliases.len())))
            .highlight_style(Style::default().bg(ratatui::style::Color::Rgb(40,44,52)))
            .highlight_symbol("▶ "),
        list, &mut app.alias_state.clone(),
    );
    f.render_widget(status("↑↓ navigate  •  Esc/q back"), bar);
}

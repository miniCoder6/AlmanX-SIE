use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style}, text::{Line, Span}, widgets::{List, ListItem, Paragraph, Wrap}};
use crate::tui::app::App;
use super::{ACCENT, DIM, GOLD, GREEN, block, status};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let vert = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)]).split(area);
    let horiz = Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(74), Constraint::Percentage(26)]).split(vert[0]);

    render_list(f, app, horiz[0]);
    render_help(f, horiz[1]);
    f.render_widget(status(&app.status), vert[1]);
}

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let (list_area, filter_area) = if app.filter.is_empty() {
        (area, None)
    } else {
        let parts = Layout::default().direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)]).split(area);
        (parts[1], Some(parts[0]))
    };

    if let Some(strip) = filter_area {
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::styled(" filter: ", Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
            Span::styled(&app.filter, Style::default().fg(ratatui::style::Color::White)),
            Span::styled("  Esc clear", Style::default().fg(DIM)),
        ])), strip);
    }

    let title = if app.filter.is_empty() {
        format!("Flux  •  {} commands", app.commands.len())
    } else {
        format!("Flux  •  {}/{}", app.visible.len(), app.commands.len())
    };

    let items: Vec<ListItem> = app.visible.iter().filter_map(|&i| {
        let r = app.commands.get(i)?;
        Some(ListItem::new(Line::from(vec![
            Span::styled(format!("{:>5} ", r.score), Style::default().fg(GREEN)),
            Span::styled(format!("×{:<3} ", r.frequency), Style::default().fg(GOLD)),
            Span::styled(r.command.clone(), Style::default().fg(ratatui::style::Color::White)),
        ])))
    }).collect();

    f.render_stateful_widget(
        List::new(items).block(block(&title))
            .highlight_style(Style::default().bg(ratatui::style::Color::Rgb(40,44,52)).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        list_area, &mut app.list_state.clone(),
    );
}

fn render_help(f: &mut Frame, area: Rect) {
    let keys = vec![
        kl("type", "filter list"),
        kl("/ s", "search"),
        kl("a", "add alias"),
        kl("r", "remove alias"),
        kl("c", "change alias"),
        kl("l", "list aliases"),
        kl("t", "stats"),
        kl("Q", "query"),
        Line::from(""),
        kl("↑↓ jk", "navigate"),
        kl("Esc", "clear filter"),
        kl("F5", "refresh"),
        kl("q", "quit"),
    ];
    f.render_widget(Paragraph::new(keys).block(block("Keys")).wrap(Wrap { trim: true }), area);
}

fn kl<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<6}", key), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(desc, Style::default().fg(DIM)),
    ])
}

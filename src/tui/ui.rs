use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use crate::tui::app::{Action, App, Mode};

const ACCENT: Color = Color::Rgb(56, 139, 253);
const GOLD:   Color = Color::Rgb(210, 153, 34);
const DIM:    Color = Color::Rgb(110, 110, 120);
const GREEN:  Color = Color::Rgb(80, 200, 100);

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.size();
    match &app.mode {
        Mode::Main           => render_main(f, app, area),
        Mode::Search         => render_search(f, app, area),
        Mode::AddAlias       => render_add_alias(f, app, area),
        Mode::PickSuggestion => render_pick_suggestion(f, app, area),
        Mode::RemoveAlias    => render_remove_alias(f, app, area),
        Mode::ChangeAlias    => render_change_alias(f, app, area),
        Mode::ListAliases    => render_list_aliases(f, app, area),
        Mode::Stats          => render_output(f, app, area, "Workflow Stats"),
        Mode::Predict        => render_output(f, app, area, "Workflow Predictions"),
        Mode::Context        => render_output(f, app, area, "Contextual Suggestions"),
        Mode::Query          => render_query(f, app, area),
        Mode::Confirm(_)     => { render_main(f, app, area); render_confirm(f, app, area); }
    }
}

fn block(title: &str) -> Block {
    Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
        .title(Span::styled(format!(" {} ", title), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
}

fn status(msg: &str) -> Paragraph {
    Paragraph::new(format!(" {}", msg)).style(Style::default().fg(Color::Black).bg(ACCENT))
}

// --- main_view.rs ---
fn render_main(f: &mut Frame, app: &App, area: Rect) {
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
            Span::styled(&app.filter, Style::default().fg(Color::White)),
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
            Span::styled(r.command.clone(), Style::default().fg(Color::White)),
        ])))
    }).collect();

    f.render_stateful_widget(
        List::new(items).block(block(&title))
            .highlight_style(Style::default().bg(Color::Rgb(40,44,52)).add_modifier(Modifier::BOLD))
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
        kl("p", "predict next"),
        kl("x", "local context"),
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

// --- search_view.rs ---
fn render_search(f: &mut Frame, app: &App, area: Rect) {
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
            .highlight_style(Style::default().bg(Color::Rgb(40,44,52)).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        chunks[1], &mut app.search_state.clone(),
    );
    f.render_widget(status("Type to search  •  ↑↓ navigate  •  Esc back"), chunks[2]);
}

// --- add_alias_view.rs ---
fn render_add_alias(f: &mut Frame, app: &App, area: Rect) {
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

// --- pick_suggestion_view.rs ---
fn render_pick_suggestion(f: &mut Frame, app: &App, area: Rect) {
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
            .highlight_style(Style::default().bg(Color::Rgb(40,44,52)).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ "),
        list, &mut app.sug_state.clone(),
    );
    f.render_widget(status("↑↓ navigate  •  Enter select  •  Esc back"), bar);
}

// --- remove_alias_view.rs ---
fn render_remove_alias(f: &mut Frame, app: &App, area: Rect) {
    let [inp, bar] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1)]).split(area)[..] else { return };
    f.render_widget(
        Paragraph::new(app.input.as_str())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(" Remove Alias ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))),
        inp,
    );
    f.render_widget(status("Type alias name  •  Enter confirm  •  Esc back"), bar);
}

// --- change_alias_view.rs ---
fn render_change_alias(f: &mut Frame, app: &App, area: Rect) {
    let [hint, inp, bar] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(1)]).split(area)[..] else { return };
    f.render_widget(Paragraph::new("old_alias  new_alias  command").block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(Span::styled(" Change Alias ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))).style(Style::default().fg(DIM)), hint);
    f.render_widget(Paragraph::new(app.input.as_str()).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)), inp);
    f.render_widget(status("old_alias new_alias command  •  Enter confirm  •  Esc back"), bar);
}

// --- list_aliases_view.rs ---
fn render_list_aliases(f: &mut Frame, app: &App, area: Rect) {
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
            .highlight_style(Style::default().bg(Color::Rgb(40,44,52)))
            .highlight_symbol("▶ "),
        list, &mut app.alias_state.clone(),
    );
    f.render_widget(status("↑↓ navigate  •  Esc/q back"), bar);
}

// --- output_view.rs ---
fn render_output(f: &mut Frame, app: &App, area: Rect, title: &str) {
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

fn render_query(f: &mut Frame, app: &App, area: Rect) {
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

// --- confirm_view.rs ---
fn render_confirm(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered(50, 28, area);
    f.render_widget(Clear, popup);

    let (title, body) = match &app.mode {
        Mode::Confirm(Action::AddAlias { alias, command }) =>
            (" Add Alias ", format!("Add:  {} = '{}'", alias, command)),
        Mode::Confirm(Action::RemoveAlias { alias }) =>
            (" Remove Alias ", format!("Remove alias: {}", alias)),
        _ => (" Confirm ", "Are you sure?".into()),
    };

    let [msg, btns] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(3)]).split(popup)[..] else { return };

    f.render_widget(
        Paragraph::new(body.as_str())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))))
            .alignment(Alignment::Center).wrap(Wrap { trim: true }),
        msg,
    );

    let yes = if app.confirm_yes { Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
    let no  = if !app.confirm_yes { Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  [ OK ]  ", yes), Span::raw("   "), Span::styled("  [ Cancel ]  ", no),
        ])).alignment(Alignment::Center).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)),
        btns,
    );
}

fn centered(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let v = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Percentage((100-pct_y)/2), Constraint::Percentage(pct_y), Constraint::Percentage((100-pct_y)/2)])
        .split(r);
    Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Percentage((100-pct_x)/2), Constraint::Percentage(pct_x), Constraint::Percentage((100-pct_x)/2)])
        .split(v[1])[1]
}

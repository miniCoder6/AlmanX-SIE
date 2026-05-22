// ─── tui/render.rs ────────────────────────────────────────────────────────────
use super::app::{App, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

const C_ACCENT:   Color = Color::Cyan;
const C_DIM:      Color = Color::DarkGray;
const C_SELECTED: Color = Color::Blue;
const C_WARN:     Color = Color::Yellow;
const C_OK:       Color = Color::Green;

fn bold(c: Color) -> Style  { Style::default().fg(c).add_modifier(Modifier::BOLD) }
fn dim()          -> Style  { Style::default().fg(C_DIM) }
fn plain(c: Color) -> Style { Style::default().fg(c) }
fn highlight()    -> Style  { Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD) }

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.size();
    match &app.mode.clone() {
        Mode::Browse | Mode::Search          => draw_main(f, app, area),
        Mode::AddAlias { command }            => draw_add_alias(f, app, area, command),
        Mode::ConfirmAlias { command, alias } => {
            draw_main(f, app, area);
            draw_confirm_popup(f, area, alias, command);
        }
        Mode::ListAliases                    => draw_list_aliases(f, app, area),
        Mode::Workflows                      => draw_workflows(f, app, area),
        Mode::Popup { message }              => {
            draw_main(f, app, area);
            draw_popup(f, area, message);
        }
    }
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Length(3), // search bar
            Constraint::Min(1),    // command list + help
            Constraint::Length(1), // status bar
        ])
        .split(area);

    // Title bar
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" AlmanX ", bold(C_ACCENT)),
        Span::styled("— shell intelligence engine", dim()),
        Span::styled(
            format!("  {} commands tracked", app.commands.len()),
            plain(C_WARN)
        ),
    ]));
    f.render_widget(title, chunks[0]);

    // Search bar
    let search_style = if app.mode == Mode::Search { bold(C_ACCENT) } else { plain(C_DIM) };
    let search_text = if app.mode == Mode::Search && !app.search.is_empty() {
        format!("{}_", app.search)
    } else if app.search.is_empty() {
        "Press / to filter commands…".to_owned()
    } else {
        app.search.clone()
    };
    let search = Paragraph::new(search_text)
        .block(Block::default().borders(Borders::ALL).title(" Search ").border_style(search_style))
        .style(plain(Color::White));
    f.render_widget(search, chunks[1]);

    // Split command list + help panel
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[2]);

    // Command list
    let items: Vec<ListItem> = app.filtered.iter().map(|cmd| {
        let freq_bar = "▪".repeat((cmd.frequency as usize).min(10));
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:>6}  ", cmd.score), dim()),
            Span::styled(format!("{:<40}", &cmd.text), plain(Color::White)),
            Span::styled(freq_bar, plain(Color::DarkGray)),
        ]))
    }).collect();

    let list_title = if app.search.is_empty() {
        format!(" Commands ({}) ", app.filtered.len())
    } else {
        format!(" Commands ({} / {}) ", app.filtered.len(), app.commands.len())
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(highlight())
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, main_chunks[0], &mut app.list_state.clone());

    // Help panel
    let help = Paragraph::new(vec![
        Line::from(vec![Span::styled("AlmanX Keys", bold(C_ACCENT))]),
        Line::from(""),
        key_line("/",      "search/filter"),
        key_line("↑ ↓",   "navigate"),
        key_line("a / ⏎", "add alias"),
        key_line("d",      "dismiss command"),
        key_line("l",      "list aliases"),
        key_line("w",      "view workflows"),
        key_line("q",      "quit"),
        Line::from(""),
        Line::from(vec![Span::styled("CLI Commands", bold(C_ACCENT))]),
        Line::from(""),
        cli_line("almanx search <q>",  "fuzzy search"),
        cli_line("almanx stats",       "analytics"),
        cli_line("almanx workflows",   "DAG patterns"),
        cli_line("almanx suggest",     "alias ideas"),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Help "))
    .wrap(Wrap { trim: true });
    f.render_widget(help, main_chunks[1]);

    // Status bar
    let status = Paragraph::new(app.status.as_str()).style(dim());
    f.render_widget(status, chunks[3]);
}

fn draw_add_alias(f: &mut Frame, app: &App, area: Rect, command: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let cmd_para = Paragraph::new(command)
        .block(Block::default().borders(Borders::ALL).title(" Command "))
        .style(plain(Color::White));
    f.render_widget(cmd_para, chunks[0]);

    let alias_text = if app.alias_input.is_empty() {
        "Type alias name or pick suggestion below…".to_owned()
    } else {
        format!("{}_", app.alias_input)
    };
    let input = Paragraph::new(alias_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Alias name (Enter to confirm) ")
                .border_style(bold(C_ACCENT)),
        )
        .style(plain(Color::White));
    f.render_widget(input, chunks[1]);

    let items: Vec<ListItem> = app.alias_suggestions.iter().map(|s| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<10}", s.alias), bold(C_OK)),
            Span::styled(&s.reason, dim()),
        ]))
    }).collect();

    let sugg_title = if items.is_empty() {
        " Suggestions (none) ".to_owned()
    } else {
        format!(" Suggestions ({}) — Tab to select ", items.len())
    };
    let sugg = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(sugg_title))
        .highlight_style(highlight())
        .highlight_symbol("▶ ");
    f.render_stateful_widget(sugg, chunks[2], &mut app.suggestions_state.clone());

    let hint = Paragraph::new("Esc=cancel   ↑↓=navigate   Tab=pick suggestion   Enter=confirm")
        .style(dim());
    f.render_widget(hint, chunks[3]);
}

fn draw_list_aliases(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let items: Vec<ListItem> = app.aliases.iter().map(|(name, cmd)| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<14}", name), bold(C_ACCENT)),
            Span::styled(cmd.as_str(), plain(Color::White)),
        ]))
    }).collect();

    let title = if items.is_empty() {
        " Aliases (none) ".to_owned()
    } else {
        format!(" Aliases ({}) ", items.len())
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(highlight())
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, chunks[0], &mut app.aliases_state.clone());

    let hint = Paragraph::new("Esc=back   ↑↓=navigate   d=delete alias").style(dim());
    f.render_widget(hint, chunks[1]);
}

fn draw_workflows(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Min(1),    // list
            Constraint::Length(1), // hint
        ])
        .split(area);

    let title_line = Paragraph::new(Line::from(vec![
        Span::styled(" Workflow Patterns ", bold(C_ACCENT)),
        Span::styled("— frequently repeated command sequences", dim()),
    ]));
    f.render_widget(title_line, chunks[0]);

    let items: Vec<ListItem> = if app.workflows.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled(
                "  No workflows found yet. Keep using your shell and come back!",
                dim()
            )
        ]))]
    } else {
        app.workflows.iter().map(|wf| {
            let chain = wf.nodes.join(" → ");
            let freq_bar = "█".repeat((wf.frequency as usize).min(20));
            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("  {:>4}×  ", wf.frequency), bold(C_WARN)),
                    Span::styled(chain, plain(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("         ", plain(Color::White)),
                    Span::styled(freq_bar, plain(Color::Green)),
                ]),
            ])
        }).collect()
    };

    let title = format!(" Workflows ({}) — ≥2 occurrences ", app.workflows.len());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(highlight())
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, chunks[1], &mut app.workflows_state.clone());

    let hint = Paragraph::new("Esc=back   ↑↓=navigate   Try: almanx add <alias> '<cmd1> && <cmd2>'")
        .style(dim());
    f.render_widget(hint, chunks[2]);
}

fn draw_confirm_popup(f: &mut Frame, area: Rect, alias: &str, command: &str) {
    let popup = centered_rect(60, 7, area);
    f.render_widget(Clear, popup);
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  alias "),
            Span::styled(alias, bold(C_OK)),
            Span::raw("='"),
            Span::styled(command, plain(Color::White)),
            Span::raw("'"),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Enter/y = confirm   Esc/n = cancel", dim())),
    ];
    let popup_widget = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Alias ")
                .border_style(bold(C_ACCENT)),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(popup_widget, popup);
}

fn draw_popup(f: &mut Frame, area: Rect, message: &str) {
    let popup = centered_rect(60, 5, area);
    f.render_widget(Clear, popup);
    let p = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {}", message), plain(Color::White))),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" AlmanX ")
            .border_style(bold(C_ACCENT)),
    );
    f.render_widget(p, popup);
}

fn key_line<'a>(key: &'a str, action: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<8}", key), bold(C_ACCENT)),
        Span::styled(action, plain(Color::White)),
    ])
}

fn cli_line<'a>(cmd: &'a str, action: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<20}", cmd), plain(C_ACCENT)),
        Span::styled(action, dim()),
    ])
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - (height * 100 / area.height.max(1))) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - (height * 100 / area.height.max(1))) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

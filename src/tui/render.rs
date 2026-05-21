// ─── tui/render.rs ────────────────────────────────────────────────────────────
//
// Pure rendering — reads from App, writes to Frame.  No state mutation here.

use super::app::{App, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

// ── Palette ───────────────────────────────────────────────────────────────────

const C_ACCENT:   Color = Color::Cyan;
const C_DIM:      Color = Color::DarkGray;
const C_SELECTED: Color = Color::Blue;
const C_WARN:     Color = Color::Yellow;
const C_OK:       Color = Color::Green;
const C_ERR:      Color = Color::Red;

fn bold(c: Color) -> Style        { Style::default().fg(c).add_modifier(Modifier::BOLD) }
fn dim()          -> Style        { Style::default().fg(C_DIM) }
fn plain(c: Color) -> Style       { Style::default().fg(c) }
fn highlight()     -> Style       { Style::default().bg(Color::DarkGray) }

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.size();

    match &app.mode.clone() {
        Mode::Browse | Mode::Search   => draw_main(f, app, area),
        Mode::AddAlias { command }     => draw_add_alias(f, app, area, command),
        Mode::ConfirmAlias { command, alias } => {
            draw_main(f, app, area);
            draw_confirm_popup(f, area, alias, command);
        }
        Mode::ListAliases              => draw_list_aliases(f, app, area),
        Mode::Popup { message }        => {
            draw_main(f, app, area);
            draw_popup(f, area, message);
        }
    }
}

// ── Main view ─────────────────────────────────────────────────────────────────

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Min(1),    // command list
            Constraint::Length(1), // status bar
        ])
        .split(area);

    // ── Search bar ────────────────────────────────────────────────────────────
    let search_style = if app.mode == Mode::Search {
        bold(C_ACCENT)
    } else {
        plain(C_DIM)
    };
    let search_text = if app.search.is_empty() {
        "Press / to search…".to_owned()
    } else {
        app.search.clone()
    };
    let search = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search ")
                .border_style(search_style),
        )
        .style(plain(Color::White));
    f.render_widget(search, chunks[0]);

    // ── Split into command list + help panel ──────────────────────────────────
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(chunks[1]);

    // Command list
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|cmd| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>5}  ", cmd.score), dim()),
                Span::styled(&cmd.text, plain(Color::White)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Commands ({}) ", app.filtered.len())),
        )
        .highlight_style(highlight().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, main_chunks[0], &mut app.list_state.clone());

    // Help panel
    let help = Paragraph::new(vec![
        Line::from(vec![Span::styled("Keys", bold(C_ACCENT))]),
        Line::from(""),
        key_line("/",     "search"),
        key_line("↑ ↓",  "navigate"),
        key_line("a / ⏎","add alias"),
        key_line("d",     "dismiss"),
        key_line("l",     "list aliases"),
        key_line("q",     "quit"),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Help "))
    .wrap(Wrap { trim: true });
    f.render_widget(help, main_chunks[1]);

    // ── Status bar ────────────────────────────────────────────────────────────
    let status = Paragraph::new(app.status.as_str()).style(dim());
    f.render_widget(status, chunks[2]);
}

// ── Add Alias view ────────────────────────────────────────────────────────────

fn draw_add_alias(f: &mut Frame, app: &App, area: Rect, command: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // command display
            Constraint::Length(3),  // alias input
            Constraint::Min(1),     // suggestions list
            Constraint::Length(1),  // hint
        ])
        .split(area);

    let cmd_para = Paragraph::new(command)
        .block(Block::default().borders(Borders::ALL).title(" Command "))
        .style(plain(Color::White));
    f.render_widget(cmd_para, chunks[0]);

    let alias_text = if app.alias_input.is_empty() {
        "Type alias name or pick suggestion below…".to_owned()
    } else {
        app.alias_input.clone()
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

    // Suggestions
    let items: Vec<ListItem> = app
        .alias_suggestions
        .iter()
        .map(|s| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<8}", s.alias), bold(C_OK)),
                Span::styled("  ", plain(Color::White)),
                Span::styled(&s.reason, dim()),
            ]))
        })
        .collect();

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

    let hint = Paragraph::new("Esc=cancel   ↑↓=navigate suggestions   Tab=pick   Enter=confirm")
        .style(dim());
    f.render_widget(hint, chunks[3]);
}

// ── List Aliases view ─────────────────────────────────────────────────────────

fn draw_list_aliases(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let items: Vec<ListItem> = app
        .aliases
        .iter()
        .map(|(name, cmd)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12}", name), bold(C_ACCENT)),
                Span::styled("  ", plain(Color::White)),
                Span::styled(cmd.as_str(), plain(Color::White)),
            ]))
        })
        .collect();

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

    let hint = Paragraph::new("Esc=back   ↑↓=navigate   d=delete").style(dim());
    f.render_widget(hint, chunks[1]);
}

// ── Overlays ──────────────────────────────────────────────────────────────────

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
        Line::from(Span::styled(
            "  Press Enter / y to confirm, Esc / n to cancel",
            dim(),
        )),
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
        Line::from(Span::styled(
            format!("  {}", message),
            plain(Color::White),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" AlmanX ")
            .border_style(bold(C_ACCENT)),
    );
    f.render_widget(p, popup);
}

// ── Layout helpers ────────────────────────────────────────────────────────────

fn key_line<'a>(key: &'a str, action: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<6}", key), bold(C_ACCENT)),
        Span::styled(action, plain(Color::White)),
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

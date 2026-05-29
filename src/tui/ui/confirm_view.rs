use ratatui::{Frame, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap}};
use crate::tui::app::{App, Mode, Action};
use super::ACCENT;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
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

use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style}, text::Span, widgets::{Block, BorderType, Borders, Paragraph}};
use crate::tui::app::App;
use super::{ACCENT, DIM, block, status};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let [hint, inp, bar] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(1)]).split(area)[..] else { return };
    f.render_widget(Paragraph::new("old_alias  new_alias  command").block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(Span::styled(" Change Alias ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))).style(ratatui::style::Style::default().fg(DIM)), hint);
    f.render_widget(Paragraph::new(app.input.as_str()).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)), inp);
    f.render_widget(status("old_alias new_alias command  •  Enter confirm  •  Esc back"), bar);
}

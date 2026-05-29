use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Modifier, Style}, text::Span, widgets::{Block, BorderType, Borders, Paragraph}};
use crate::tui::app::App;
use super::{ACCENT, status};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let [inp, bar] = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1)]).split(area)[..] else { return };
    f.render_widget(
        Paragraph::new(app.input.as_str())
            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(Span::styled(" Remove Alias ", Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD)))),
        inp,
    );
    f.render_widget(status("Type alias name  •  Enter confirm  •  Esc back"), bar);
}

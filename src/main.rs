use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{
        self, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    },
};

use ratatui::{
    Terminal,
    backend::{self, CrosstermBackend},
    layout::Rect,
    prelude::{Buffer, Widget},
    style::{Color, Style},
    widgets::{Block, Borders},
};

use std::io;

struct TerminalCanvas<'a> {
    pixels: &'a [(u16, u16)],
}

impl<'a> Widget for TerminalCanvas<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Block::default()
            .borders(Borders::ALL)
            .title("Peak Project ikr haha (press 'q' to exit)")
            .render(area, buf);

        for &(x, y) in self.pixels {
            if x > area.left() && x < area.right() - 1 && y > area.top() && y < area.bottom() - 1 {
                buf.get_mut(x, y)
                    .set_symbol("█")
                    .set_style(Style::default().fg(Color::Red));
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut drawn_pixels: Vec<(u16, u16)> = Vec::new();

    loop {
        terminal.draw(|f| {
            let canvas = TerminalCanvas {
                pixels: &drawn_pixels,
            };
            f.render_widget(canvas, f.size());
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }

                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::Drag(crossterm::event::MouseButton::Left)
                    | MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                        drawn_pixels.push((mouse_event.column, mouse_event.row));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

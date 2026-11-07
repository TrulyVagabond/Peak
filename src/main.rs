use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    prelude::{Buffer, Widget},
    style::{Color, Style},
    widgets::{Block, Borders},
};

use std::io;

#[derive(PartialEq)]
enum AppMode {
    Freehand,
    Rectangle,
    Square,
}

struct TerminalCanvas<'a> {
    pixels: &'a [(u16, u16)],
    mode: &'a AppMode,
}

impl<'a> Widget for TerminalCanvas<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_name = match self.mode {
            AppMode::Freehand => "Freehand",
            AppMode::Rectangle => "Rectangle",
            AppMode::Square => "Square",
        };

        let title = format!(
            " Peak Project IKR - {} Mode (press 'q' to ragequit) ",
            mode_name
        );

        Block::default()
            .borders(Borders::ALL)
            .title(title.as_str())
            .render(area, buf);

        for &(x, y) in self.pixels {
            if x > area.left() && x < area.right() - 1 && y > area.top() && y < area.bottom() - 1 {
                buf[(x, y)]
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
    let mut current_mode = AppMode::Freehand;
    let mut shape_start: Option<(u16, u16)> = None;

    loop {
        terminal.draw(|f| {
            let canvas = TerminalCanvas {
                pixels: &drawn_pixels,
                mode: &current_mode,
            };
            f.render_widget(canvas, f.area());
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('z') => current_mode = AppMode::Freehand,
                    KeyCode::Char('x') => current_mode = AppMode::Rectangle,
                    KeyCode::Char('c') => current_mode = AppMode::Square,
                    _ => {}
                },

                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                        shape_start = Some((mouse_event.column, mouse_event.row));
                    }

                    MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                        if current_mode == AppMode::Freehand {
                            drawn_pixels.push((mouse_event.column, mouse_event.row));
                        }
                    }

                    MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                        if current_mode == AppMode::Rectangle {
                            if let Some((start_x, start_y)) = shape_start {
                                let end_x = mouse_event.column;
                                let end_y = mouse_event.row;

                                let min_x = start_x.min(end_x);
                                let max_x = start_x.max(end_x);
                                let min_y = start_y.min(end_y);
                                let max_y = start_y.max(end_y);

                                for x in min_x..=max_x {
                                    for y in min_y..=max_y {
                                        drawn_pixels.push((x, y));
                                    }
                                }
                            }
                        }
                        shape_start = None;
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

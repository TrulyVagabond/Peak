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
    Eraser,
}

struct TerminalCanvas<'a> {
    pixels: &'a [(u16, u16)],
    mode: &'a AppMode,
    ghost_pixels: &'a [(u16, u16)],
}

impl<'a> Widget for TerminalCanvas<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_name = match self.mode {
            AppMode::Freehand => "Freehand",
            AppMode::Rectangle => "Rectangle",
            AppMode::Eraser => "Eraser",
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

        for &(x, y) in self.ghost_pixels {
            if x > area.left() && x < area.right() - 1 && y > area.top() && y < area.bottom() - 1 {
                buf[(x, y)]
                    .set_symbol("░")
                    .set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }
}

fn rectangle_pixels(start: (u16, u16), end: (u16, u16)) -> Vec<(u16, u16)> {
    let mut dots: Vec<(u16, u16)> = Vec::new();
    let (start_x, start_y) = start;
    let (end_x, end_y) = end;

    let min_x = start_x.min(end_x);
    let max_x = start_x.max(end_x);
    let min_y = start_y.min(end_y);
    let max_y = start_y.max(end_y);

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            dots.push((x, y));
        }
    }

    dots
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut drawn_pixels: Vec<(u16, u16)> = Vec::new();
    let mut ghost_pixels: Vec<(u16, u16)> = Vec::new();
    let mut current_mode = AppMode::Freehand;
    let mut shape_start: Option<(u16, u16)> = None;

    loop {
        terminal.draw(|f| {
            let canvas = TerminalCanvas {
                pixels: &drawn_pixels,
                mode: &current_mode,
                ghost_pixels: &ghost_pixels,
            };
            f.render_widget(canvas, f.area());
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('f') => {
                        current_mode = AppMode::Freehand;
                        ghost_pixels.clear();
                    }
                    KeyCode::Char('r') => {
                        current_mode = AppMode::Rectangle;
                        ghost_pixels.clear();
                    }
                    KeyCode::Char('e') => {
                        current_mode = AppMode::Eraser;
                        ghost_pixels.clear();
                    }
                    KeyCode::Esc => {
                        drawn_pixels.clear();
                        ghost_pixels.clear();
                        shape_start = None;
                    }
                    _ => {}
                },

                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                        shape_start = Some((mouse_event.column, mouse_event.row));
                    }

                    MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                        let mx = mouse_event.column;
                        let my = mouse_event.row;

                        if current_mode == AppMode::Freehand {
                            drawn_pixels.push((mx, my));
                        } else if current_mode == AppMode::Eraser {
                            drawn_pixels.retain(|&(px, py)| px != mx || py != my);
                        } else if current_mode == AppMode::Rectangle {
                            if let Some((start_x, start_y)) = shape_start {
                                ghost_pixels = rectangle_pixels((start_x, start_y), (mx, my));
                            }
                        }
                    }

                    MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                        if current_mode == AppMode::Rectangle {
                            if let Some((start_x, start_y)) = shape_start {
                                let end_x = mouse_event.column;
                                let end_y = mouse_event.row;
                                drawn_pixels
                                    .extend(rectangle_pixels((start_x, start_y), (end_x, end_y)));
                            }
                        }
                        shape_start = None;
                        ghost_pixels.clear();
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

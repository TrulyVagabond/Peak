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
    Eraser,
}

struct PanicProof;

impl Drop for PanicProof {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            crossterm::cursor::Show
        );
    }
}

struct TerminalCanvas<'a> {
    pixels: &'a [Pixels],
    mode: &'a AppMode,
    ghost_pixels: &'a [(u16, u16)],
    hide_right_border: bool,
}

impl<'a> Widget for TerminalCanvas<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_name = match self.mode {
            AppMode::Freehand => "Freehand",
            AppMode::Rectangle => "Rectangle",
            AppMode::Eraser => "Eraser",
            AppMode::Square => "Square",
        };

        let borders = if self.hide_right_border {
            Borders::TOP | Borders::BOTTOM | Borders::LEFT
        } else {
            Borders::ALL
        };

        let title = format!(" PeakTUI - {} Mode (press 'q' to ragequit) ", mode_name);

        Block::default()
            .borders(borders)
            .title(title.as_str())
            .render(area, buf);

        for pixel in self.pixels {
            if pixel.x > area.left()
                && pixel.x < area.right() - 1
                && pixel.y > area.top()
                && pixel.y < area.bottom() - 1
            {
                buf[(pixel.x, pixel.y)]
                    .set_symbol("█")
                    .set_style(Style::default().fg(pixel.color));
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

struct Pixels {
    x: u16,
    y: u16,
    color: Color,
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

fn square_math(start: (u16, u16), mouse: (u16, u16)) -> (u16, u16) {
    let (start_x, start_y) = start;
    let (mx, my) = mouse;
    let dx = mx as i32 - start_x as i32;
    let dy = my as i32 - start_y as i32;
    let size = dx.abs().max(dy.abs() * 2);
    let corner_x = if dx >= 0 {
        start_x as i32 + size
    } else {
        start_x as i32 - size
    };

    let corner_y = if dy >= 0 {
        start_y as i32 + size / 2
    } else {
        start_y as i32 - size / 2
    };

    (corner_x.max(0) as u16, corner_y.max(0) as u16)
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut drawn_pixels: Vec<Pixels> = Vec::new();
    let mut ghost_pixels: Vec<(u16, u16)> = Vec::new();
    let mut current_mode = AppMode::Freehand;
    let mut sidebar_open = false;
    let mut sidebar_width: u16 = 35;
    let mut current_color = Color::Red;
    let mut shape_start: Option<(u16, u16)> = None;
    let _guard = PanicProof;

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let canvas = TerminalCanvas {
                pixels: &drawn_pixels,
                mode: &current_mode,
                ghost_pixels: &ghost_pixels,
                hide_right_border: sidebar_open,
            };

            if sidebar_open {
                let canvas_area =
                    Rect::new(area.x, area.y, area.width - sidebar_width, area.height);
                let sidebar_area = Rect::new(
                    area.right() - sidebar_width,
                    area.y,
                    sidebar_width,
                    area.height,
                );
                f.render_widget(canvas, canvas_area);
                f.render_widget(
                    Block::default()
                        .borders(Borders::TOP | Borders::BOTTOM | Borders::LEFT)
                        .title(" Color Customization "),
                    sidebar_area,
                );
            } else {
                f.render_widget(canvas, area);
            }
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
                    KeyCode::Char('s') => {
                        current_mode = AppMode::Square;
                        ghost_pixels.clear();
                    }
                    KeyCode::Char('c') => {
                        sidebar_open = !sidebar_open;
                        ghost_pixels.clear();
                    }
                    KeyCode::Char('1') => current_color = Color::Red,
                    KeyCode::Char('2') => current_color = Color::Green,
                    KeyCode::Esc => {
                        drawn_pixels.clear();
                        ghost_pixels.clear();
                        shape_start = None;
                    }
                    _ => {}
                },

                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                        let mx = mouse_event.column;
                        let my = mouse_event.row;
                        let screen = terminal.size()?;
                        let sidebar = Rect::new(
                            screen.width - sidebar_width,
                            0,
                            sidebar_width,
                            screen.height,
                        );
                        if sidebar_open && mx == sidebar.x {
                            let sidebar_drag = true;
                        }
                        let on_panel = sidebar_open
                            && mx >= sidebar.x
                            && mx < sidebar.right()
                            && my >= sidebar.y
                            && my < sidebar.bottom();
                        if sidebar_drag {
                            sidebar_width = (screen.width - mx).max(25).min(screen.width / 2)
                        } else if !on_panel {
                            shape_start = Some((mx, my));
                            if current_mode == AppMode::Freehand {
                                drawn_pixels.push(Pixels {
                                    x: mx,
                                    y: my,
                                    color: current_color,
                                });
                            } else if current_mode == AppMode::Eraser {
                                drawn_pixels.retain(|p| p.x != mx || p.y != my);
                            }
                        }
                    }

                    MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                        let mx = mouse_event.column;
                        let my = mouse_event.row;

                        let screen = terminal.size()?;
                        let sidebar = Rect::new(
                            screen.width - sidebar_width,
                            0,
                            sidebar_width,
                            screen.height,
                        );
                        if sidebar_open && mx == sidebar.x {
                            let sidebar_drag = true;
                        }
                        let on_panel = sidebar_open
                            && mx >= sidebar.x
                            && mx < sidebar.right()
                            && my >= sidebar.y
                            && my < sidebar.bottom();
                        if sidebar_drag {
                            sidebar_width = (screen.width - mx).max(25).min(sidebar_width / 2);
                        } else if !on_panel {
                            if current_mode == AppMode::Freehand {
                                drawn_pixels.push(Pixels {
                                    x: mx,
                                    y: my,
                                    color: current_color,
                                });
                            } else if current_mode == AppMode::Eraser {
                                drawn_pixels.retain(|p| p.x != mx || p.y != my);
                            } else if current_mode == AppMode::Rectangle {
                                if let Some((start_x, start_y)) = shape_start {
                                    ghost_pixels = rectangle_pixels((start_x, start_y), (mx, my));
                                }
                            } else if current_mode == AppMode::Square {
                                if let Some((start_x, start_y)) = shape_start {
                                    let (end_x, end_y) = square_math((start_x, start_y), (mx, my));
                                    ghost_pixels =
                                        rectangle_pixels((start_x, start_y), (end_x, end_y));
                                }
                            }
                        }
                    }

                    MouseEventKind::Up(crossterm::event::MouseButton::Left) => {
                        if current_mode == AppMode::Rectangle {
                            if let Some((start_x, start_y)) = shape_start {
                                let mx = mouse_event.column;
                                let my = mouse_event.row;
                                for (x, y) in rectangle_pixels((start_x, start_y), (mx, my)) {
                                    drawn_pixels.push(Pixels {
                                        x,
                                        y,
                                        color: current_color,
                                    });
                                }
                            }
                        } else if current_mode == AppMode::Square {
                            if let Some((start_x, start_y)) = shape_start {
                                let mx = mouse_event.column;
                                let my = mouse_event.row;
                                let (end_x, end_y) = square_math((start_x, start_y), (mx, my));
                                for (x, y) in rectangle_pixels((start_x, start_y), (end_x, end_y)) {
                                    drawn_pixels.push(Pixels {
                                        x,
                                        y,
                                        color: current_color,
                                    });
                                }
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

    Ok(())
}

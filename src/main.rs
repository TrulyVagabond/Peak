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
    style::{Color, Style, Styled},
    widgets::{Block, BorderType::Rounded, Borders},
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
            .set_style(Color::Rgb(255, 100, 100))
            .render(area, buf);

        for pixel in self.pixels {
            if pixel.x > area.left()
                && pixel.x < area.right() - 1
                && pixel.y > area.top()
                && pixel.y < area.bottom() - 1
            {
                buf[(pixel.x, pixel.y)]
                    .set_symbol("█")
                    .set_style(Style::default().fg(Color::Rgb(
                        pixel.color.0,
                        pixel.color.1,
                        pixel.color.2,
                    )));
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
    color: (u8, u8, u8),
}

struct SidebarLayout {
    current_strip: Rect,
    red_slider: Rect,
    green_slider: Rect,
    blue_slider: Rect,
}

fn sidebar_layout(sidebar_area: Rect) -> SidebarLayout {
    let current_color_strip = Rect::new(
        sidebar_area.x + 18,
        sidebar_area.y + 2,
        sidebar_area.width - 21,
        1,
    );
    let red_slider_strip = Rect::new(
        sidebar_area.x + 12,
        sidebar_area.y + 6,
        sidebar_area.width - 15,
        1,
    );
    let green_slider_strip = Rect::new(
        sidebar_area.x + 12,
        sidebar_area.y + 7,
        sidebar_area.width - 15,
        1,
    );
    let blue_slider_strip = Rect::new(
        sidebar_area.x + 12,
        sidebar_area.y + 8,
        sidebar_area.width - 15,
        1,
    );
    SidebarLayout {
        current_strip: current_color_strip,
        red_slider: red_slider_strip,
        green_slider: green_slider_strip,
        blue_slider: blue_slider_strip,
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
    let mut sidebar_drag = false;
    let mut mouse_pos_glow = (0u16, 0u16);
    let mut current_color: (u8, u8, u8) = (0, 0, 0);
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
                        .border_type(Rounded)
                        .borders(Borders::TOP | Borders::BOTTOM | Borders::LEFT)
                        .title(" [Color Customization] ")
                        .set_style(Color::Rgb(255, 100, 100)),
                    sidebar_area,
                );
                let layout = sidebar_layout(sidebar_area);
                let current_color_strip = layout.current_strip;
                let buf = f.buffer_mut();
                let label = "[Current Color]:";
                let label_x = sidebar_area.x + 2;
                let label_y = sidebar_area.y + 2;
                for (i, ch) in label.chars().enumerate() {
                    buf[(label_x + i as u16, label_y)]
                        .set_symbol(&ch.to_string())
                        .set_fg(Color::Rgb(255, 100, 100));
                }
                for current_color_x in current_color_strip.x..current_color_strip.right() {
                    for current_color_y in current_color_strip.y..current_color_strip.bottom() {
                        buf[(current_color_x, current_color_y)].set_bg(Color::Rgb(
                            current_color.0,
                            current_color.1,
                            current_color.2,
                        ));
                    }
                }
                let label = "[Sliders]";
                let label_x = sidebar_area.x + 2;
                let label_y = sidebar_area.y + 4;
                for (i, ch) in label.chars().enumerate() {
                    buf[(label_x + i as u16, label_y)]
                        .set_symbol(&ch.to_string())
                        .set_fg(Color::Rgb(255, 100, 100));
                }
                let label = "[Red]:";
                let label_x = sidebar_area.x + 2;
                let label_y = sidebar_area.y + 6;
                for (i, ch) in label.chars().enumerate() {
                    buf[(label_x + i as u16, label_y)]
                        .set_symbol(&ch.to_string())
                        .set_fg(Color::Rgb(255, 0, 0));
                }
                let slider_count_red = current_color.0 as u16 * layout.red_slider.width / 255;
                let red_slider = layout.red_slider;
                for red_slider_x in red_slider.x..red_slider.right() {
                    for red_slider_y in red_slider.y..red_slider.bottom() {
                        buf[(red_slider_x, red_slider_y)].set_bg(Color::Rgb(255, 0, 0));
                    }
                }
                let label = "[Green]:";
                let label_x = sidebar_area.x + 2;
                let label_y = sidebar_area.y + 7;
                for (i, ch) in label.chars().enumerate() {
                    buf[(label_x + i as u16, label_y)]
                        .set_symbol(&ch.to_string())
                        .set_fg(Color::Rgb(0, 255, 0));
                }
                let slider_count_green = current_color.1 as u16 * layout.green_slider.width / 255;
                let green_slider = layout.green_slider;
                for green_slider_x in green_slider.x..green_slider.right() {
                    for green_slider_y in green_slider.y..green_slider.bottom() {
                        buf[(green_slider_x, green_slider_y)].set_bg(Color::Rgb(0, 255, 0));
                    }
                }
                let label = "[Blue]:";
                let label_x = sidebar_area.x + 2;
                let label_y = sidebar_area.y + 8;
                for (i, ch) in label.chars().enumerate() {
                    buf[(label_x + i as u16, label_y)]
                        .set_symbol(&ch.to_string())
                        .set_fg(Color::Rgb(0, 0, 255));
                }
                let slider_count_blue = current_color.2 as u16 * layout.blue_slider.width / 255;
                let blue_slider = layout.blue_slider;
                for blue_slider_x in blue_slider.x..blue_slider.right() {
                    for blue_slider_y in blue_slider.y..blue_slider.bottom() {
                        buf[(blue_slider_x, blue_slider_y)].set_bg(Color::Rgb(0, 0, 255));
                    }
                }
                let (mouse_x, _mouse_y) = mouse_pos_glow;
                let glow = mouse_x == sidebar_area.x;
                let grip_color = if glow { Color::Red } else { Color::DarkGray };
                if sidebar_area.height >= 5 {
                    let mid = sidebar_area.top() + sidebar_area.height / 2;
                    for row in (mid - 2)..=(mid + 2) {
                        buf[(sidebar_area.x, row)]
                            .set_symbol("⣿")
                            .set_style(Style::default().fg(grip_color));
                    }
                }
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
                    KeyCode::Char('1') => current_color = (0, 0, 0),
                    KeyCode::Char('2') => current_color = (255, 0, 0),
                    KeyCode::Char('3') => current_color = (0, 255, 0),
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
                            sidebar_drag = true;
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

                        let on_panel = sidebar_open
                            && mx >= sidebar.x
                            && mx < sidebar.right()
                            && my >= sidebar.y
                            && my < sidebar.bottom();
                        if sidebar_drag {
                            sidebar_width = (screen.width - mx).max(25).min(screen.width / 2);
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

                    MouseEventKind::Moved => {
                        mouse_pos_glow = (mouse_event.column, mouse_event.row);
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
                        sidebar_drag = false;
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

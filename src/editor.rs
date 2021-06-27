use crate::Document;
use crate::Row;
use crate::Terminal;
use std::env;
use termion::color;
use termion::event::Key;

const STATUS_BG_COLOR: color::Rgb = color::Rgb(  0,   0,   0);
const PAPER_BG_COLOR:  color::Rgb = color::Rgb( 20,  20,  20);
const PAPER_WIDTH: usize = 80;
const NUM_WIDTH: usize = 5;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}
pub struct Editor {
    should_quit:     bool,
    terminal:        Terminal,
    cursor_position: Position,
    offset:          Position,
    document:        Document,
}

impl Editor {
    pub fn run(&mut self) {
        loop {
            if let Err(error) = self.refresh_screen() {
                die(&error);
            }
            
            if self.should_quit { break; }

            if let Err(error) = self.process_keypress() {
                die(&error);
            }
        }
    }

    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let document = if args.len() > 1 {
            let file_name = &args[1];
            Document::open(&file_name).unwrap_or_default()
        } else {
            Document::default()
        };

        Self {
            should_quit: false,
            terminal: Terminal::default().expect("Terminal init failed."),
            document,
            cursor_position: Position::default(),
            offset: Position::default(),
        }
    }

    fn refresh_screen(&self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());

        if self.should_quit {
            Terminal::clear_screen();
            print!("cutted exiting.\r\n")
        } else {
            self.draw_rows();
            self.draw_status_bar();
            Terminal::cursor_position(&Position {
                x: self.cursor_position.x.saturating_sub(self.offset.x),
                y: self.cursor_position.y.saturating_sub(self.offset.y),
            });
        }

        Terminal::cursor_show();
        Terminal::flush()
    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let pressed_key = Terminal::read_key()?;
        match pressed_key {
            Key::Ctrl('c') => self.should_quit = true,
            Key::Up       |
            Key::Down     |
            Key::Left     |
            Key::Right    |
            Key::PageUp   |
            Key::PageDown |
            Key::End      |
            Key::Home     => self.move_cursor(pressed_key),
            _ => (),
        }
        self.scroll();
        Ok(())
    }

    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let width  = self.terminal.size().width  as usize;
        // Subtract 1 for status line:
        let height = self.terminal.size().height as usize;
        let mut offset = &mut self.offset;

        if y < offset.y {
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            offset.y = y.saturating_sub(height).saturating_add(1);
        }

        if x < offset.x {
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }

    fn move_cursor(&mut self, key: Key) {
        let terminal_height = self.terminal.size().height as usize;
        let Position { mut y, mut x } = self.cursor_position;
        let height = self.document.len().saturating_sub(1);
        let mut width = self.document.row(y).map_or(0, Row::len);
        //let mut width = if let Some(row) = self.document.row(y) {
        //    row.len()
        //} else {
        //    0
        //};

        match key {
            Key::Up   => y = y.saturating_sub(1),
            Key::Down => if y < height { y = y.saturating_add(1) },
            Key::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;
                    if let Some(row) = self.document.row(y) {
                        x = row.len();
                    } else {
                        x = 0;
                    }
                }
            }
            Key::Right => {
                if x < width {
                    x += 1;
                } else if y < height {
                    y += 1;
                    x = 0;
                }
            }
            Key::PageUp => {
                y = if y > terminal_height {
                    y - terminal_height
                } else {
                    0
                }
            }
            Key::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y + terminal_height as usize
                } else {
                    height
                }
            }
            Key::Home => x = 0,
            Key::End  => x = width,
            _ => (),
        }

        width = self.document.row(y).map_or(0, Row::len);
        // width = if let Some(row) = self.document.row(y) {
        //     row.len()
        // } else {
        //     0
        // };

        if x > width { x = width; }

        self.cursor_position = Position { x, y }
    }

    fn draw_welcome_message(&self) {
        let mut welcome_message = format!("cutted v.{}", VERSION);
        let width = self.terminal.size().width as usize;
        let len = welcome_message.len();
        let padding = width.saturating_sub(len) / 2;
        let spacer = " ".repeat(padding.saturating_sub(1));
        welcome_message = format!("~{}{}", spacer, welcome_message);
        welcome_message.truncate(width);
        print!("{}\r\n", welcome_message);
    }

    pub fn draw_row(&self, row: &Row) {
        let width = self.terminal.size().width as usize;
        let start = self.offset.x;
        let end   = self.offset.x + width;
        let len   = row.len();

        if start < PAPER_WIDTH {
            Terminal::set_bg_color(PAPER_BG_COLOR);
            print!("{}", row.render(start, PAPER_WIDTH));
            if start > len {
                print!("{}", " ".repeat(PAPER_WIDTH.saturating_sub(start)));
            } else {
                print!("{}", " ".repeat(PAPER_WIDTH.saturating_sub(len)));
            }
            Terminal::reset_bg_color();
            print!("{}\r\n", row.render(PAPER_WIDTH, end));
        } else {
            print!("{}\r\n", row.render(start, end));
        }
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;
        for terminal_row in 0..height {
            Terminal::clear_current_line();

            if let Some(row) = self.document.row(terminal_row as usize
                                               + self.offset.y) {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 2 {
                 self.draw_welcome_message();
            } else {
                print!("~\r\n");
            }
        }
    }

    fn draw_status_bar(&self) {
        let spacer = "|".repeat(self.terminal.size().width as usize);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        print!("{}", spacer);
        Terminal::reset_bg_color();
    }
}

fn die(e: &std::io::Error) {
    Terminal::clear_screen();
    panic!("{}", e);
}

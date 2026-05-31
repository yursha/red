use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};

struct Editor {
    lines: Vec<String>,
    cursor_x: usize,
    cursor_y: usize,
    should_quit: bool,
}

impl Editor {
    fn new() -> Self {
        Self {
            lines: vec![String::new()], // Start with one empty line
            cursor_x: 0,
            cursor_y: 0,
            should_quit: false,
        }
    }

    fn run(&mut self) -> io::Result<()> {
        // Step 1: Initialize terminal state and enter Raw Mode
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Show)?;

        // Step 2: Core Render and Input Loop
        while !self.should_quit {
            self.refresh_screen(&mut stdout)?;
            self.process_keypress()?;
        }

        // Step 3: Clean up and restore terminal state on exit
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn refresh_screen(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        // Clear the screen and reset the cursor position to top-left
        execute!(stdout, cursor::Hide, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        // Draw the text buffer lines
        for (i, line) in self.lines.iter().enumerate() {
            write!(stdout, "{}", line)?;
            if i < self.lines.len() - 1 {
                write!(stdout, "\r\n")?;
            }
        }

        // Reposition the physical terminal cursor to match our virtual editor state
        execute!(
            stdout,
            cursor::MoveTo(self.cursor_x as u16, self.cursor_y as u16),
            cursor::Show
        )?;
        stdout.flush()
    }

    fn process_keypress(&mut self) -> io::Result<()> {
        // Block execution until a key event occurs
        if let Event::Key(key_event) = event::read()? {
            match (key_event.code, key_event.modifiers) {
                // Global Controls
                (KeyCode::Char('q'), KeyModifiers::CONTROL) => self.should_quit = true,

                // Navigation Controls
                (KeyCode::Left, _) => {
                    if self.cursor_x > 0 { self.cursor_x -= 1; }
                }
                (KeyCode::Right, _) => {
                    if self.cursor_x < self.lines[self.cursor_y].len() { self.cursor_x += 1; }
                }
                (KeyCode::Up, _) => {
                    if self.cursor_y > 0 {
                        self.cursor_y -= 1;
                        self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
                    }
                }
                (KeyCode::Down, _) => {
                    if self.cursor_y < self.lines.len() - 1 {
                        self.cursor_y += 1;
                        self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
                    }
                }

                // Text Editing Controls
                (KeyCode::Char(c), _) => {
                    self.lines[self.cursor_y].insert(self.cursor_x, c);
                    self.cursor_x += 1;
                }
                (KeyCode::Enter, _) => {
                    let current_line = &mut self.lines[self.cursor_y];
                    let remainder = current_line.split_off(self.cursor_x);
                    self.lines.insert(self.cursor_y + 1, remainder);
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                }
                (KeyCode::Backspace, _) => {
                    if self.cursor_x > 0 {
                        self.lines[self.cursor_y].remove(self.cursor_x - 1);
                        self.cursor_x -= 1;
                    } else if self.cursor_y > 0 {
                        // Merge current line with the line above it
                        let current_line = self.lines.remove(self.cursor_y);
                        self.cursor_y -= 1;
                        self.cursor_x = self.lines[self.cursor_y].len();
                        self.lines[self.cursor_y].push_str(&current_line);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut editor = Editor::new();
    editor.run()
}

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
    row_offset: usize,
    should_quit: bool,
    filename: Option<String>,
}

impl Editor {
    fn new() -> Self {
        Self {
            lines: vec![String::new()], // Start with one empty line
            cursor_x: 0,
            cursor_y: 0,
            row_offset: 0,
            should_quit: false,
            filename: None,
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
            self.handle_input()?;
        }

        // Step 3: Clean up and restore terminal state on exit
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn refresh_screen(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        let (_, rows) = terminal::size()?;
        let screen_rows = rows - 1; // Reserve one line for status bar

        self.scroll(screen_rows);

        // Clear the screen and reset the cursor position to top-left
        execute!(
            stdout,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        // Only draw the range of lines visible in the viewport
        let end = (self.row_offset + screen_rows as usize).min(self.lines.len());
        // Draw the text buffer lines
        for i in self.row_offset..end {
            write!(stdout, "{}\r\n", self.lines[i])?;
        }

        // Adjust cursor position to be relative to the viewport
        let relative_y = (self.cursor_y - self.row_offset) as u16;
        execute!(
            stdout,
            cursor::MoveTo(self.cursor_x as u16, relative_y),
            cursor::Show
        )?;
        stdout.flush()
    }

    fn scroll(&mut self, screen_rows: u16) {
        // If cursor is above the visible area, scroll up
        if self.cursor_y < self.row_offset {
            self.row_offset = self.cursor_y;
        }
        // If cursor is below the visible area, scroll down
        if self.cursor_y >= self.row_offset + screen_rows as usize {
            self.row_offset = self.cursor_y - screen_rows as usize + 1;
        }
    }

    fn handle_input(&mut self) -> io::Result<()> {
        // Block execution until an event occurs
        match event::read()? {
            Event::Key(key_event) => {
                match (key_event.code, key_event.modifiers) {
                    // Global Controls
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => self.should_quit = true,

                    (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                        self.save_file().expect("Failed to save");
                    }

                    // Navigation Controls
                    (KeyCode::Left, _) => {
                        if self.cursor_x > 0 {
                            self.cursor_x -= 1;
                        }
                    }
                    (KeyCode::Right, _) => {
                        if self.cursor_x < self.lines[self.cursor_y].len() {
                            self.cursor_x += 1;
                        }
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
            Event::Paste(text) => {
                for c in text.chars() {
                    if c == '\n' {
                        // Handle newlines within the paste
                        let current_line = &mut self.lines[self.cursor_y];
                        let remainder = current_line.split_off(self.cursor_x);
                        self.lines.insert(self.cursor_y + 1, remainder);
                        self.cursor_y += 1;
                        self.cursor_x = 0;
                    } else {
                        self.lines[self.cursor_y].insert(self.cursor_x, c);
                        self.cursor_x += 1;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    // Load file contents into the buffer
    fn load_file(&mut self, path: &str) -> io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.lines = content.lines().map(|s| s.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.filename = Some(path.to_string());
        Ok(())
    }

    // Save current buffer to disk
    fn save_file(&self) -> io::Result<()> {
        if let Some(ref path) = self.filename {
            let content = self.lines.join("\n");
            std::fs::write(path, content)?;
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut editor = Editor::new();

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let file_path = &args[1];
        if let Err(e) = editor.load_file(file_path) {
            eprintln!("Error opening file: {}", e);
            // Decide if you want to exit or start empty
            std::process::exit(1);
        }
    }

    editor.run()
}

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, terminal,
};
use std::fs;
use std::io::{self, Write};
use std::io::Result;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Insert,
    Command,
}

#[derive(Clone, Copy)]
struct Position {
    x: usize,
    y: usize,
}

#[derive(Clone)]
struct EditorState {
    buffer: Vec<String>,
    cursor: Position,
    filename: Option<String>,
    dirty: bool,
}

struct Editor {
    state: EditorState,
    mode: Mode,
    command: String,
    undo_stack: Vec<EditorState>,
    redo_stack: Vec<EditorState>,
    confirm_exit: bool,
    pending_save: bool,
    clipboard: String,
    ask_filename: bool,
    input_filename: String,
}

impl Editor {
    fn new(filename: Option<String>) -> Self {
        let buffer = Self::load_file(&filename);
        Self {
            state: EditorState {
                buffer,
                cursor: Position { x: 0, y: 0 },
                filename,
                dirty: false,
            },
            mode: Mode::Insert,
            command: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            confirm_exit: false,
            pending_save: false,
            clipboard: String::new(),
            ask_filename: false,
            input_filename: String::new(),
        }
    }

    fn load_file(filename: &Option<String>) -> Vec<String> {
        if let Some(file) = filename {
            fs::read_to_string(file)
                .unwrap_or_default()
                .lines()
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![String::new()]
        }
    }

    fn clamp_cursor(&mut self) {
        if self.state.buffer.is_empty() {
            self.state.cursor = Position { x: 0, y: 0 };
            return;
        }
        self.state.cursor.y =
            self.state.cursor.y.min(self.state.buffer.len().saturating_sub(1));
        self.state.cursor.x =
            self.state.cursor.x.min(self.state.buffer[self.state.cursor.y].len());
    }

    fn save_snapshot(&mut self) {
        self.redo_stack.clear();
        self.undo_stack.push(self.state.clone());
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.state.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.state.clone());
            self.state = prev;
            self.clamp_cursor();
        }
    }

    fn insert(&mut self, c: char) {
        self.save_snapshot();
        let line = &mut self.state.buffer[self.state.cursor.y];

        if let Some(pair) = Self::matching_pair(c) {
            line.insert(self.state.cursor.x, c);
            line.insert(self.state.cursor.x + 1, pair);
            self.state.cursor.x += 1;
        } else {
            line.insert(self.state.cursor.x, c);
            self.state.cursor.x += 1;
        }
        self.clamp_cursor();
    }

    fn delete(&mut self) {
        if self.state.cursor.x == 0 && self.state.cursor.y == 0 {
            return;
        }
        self.save_snapshot();

        if self.state.cursor.x > 0 {
            let line = &mut self.state.buffer[self.state.cursor.y];
            line.remove(self.state.cursor.x - 1);
            self.state.cursor.x -= 1;
        } else {
            let y = self.state.cursor.y;
            let prev_len = self.state.buffer[y - 1].len();
            let line = self.state.buffer.remove(y);
            self.state.buffer[y - 1].push_str(&line);
            self.state.cursor.y -= 1;
            self.state.cursor.x = prev_len;
        }
    }

    fn newline(&mut self) {
        self.save_snapshot();
        let y = self.state.cursor.y;
        let rest = self.state.buffer[y].split_off(self.state.cursor.x);
        self.state.buffer.insert(y + 1, rest);
        self.state.cursor.y += 1;
        self.state.cursor.x = 0;
    }

    fn copy_selection(&mut self) {
        let line = &self.state.buffer[self.state.cursor.y];
        self.clipboard = line.clone();
    }

    fn paste(&mut self) {
        if !self.clipboard.is_empty() {
            self.save_snapshot();
            let line = &mut self.state.buffer[self.state.cursor.y];
            line.insert_str(self.state.cursor.x, &self.clipboard);
            self.state.cursor.x += self.clipboard.len();
        }
    }

    fn save_to_file(&mut self, filename: String) -> Result<()> {
        fs::write(&filename, self.state.buffer.join("\n"))?;
        self.state.filename = Some(filename);
        self.state.dirty = false;
        Ok(())
    }

    fn render(&self, stdout: &mut io::Stdout) -> Result<()> {
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        for (i, line) in self.state.buffer.iter().enumerate() {
            execute!(stdout, cursor::MoveTo(0, i as u16))?;
            if i == self.state.cursor.y {
                let mut display = line.clone();
                if self.state.cursor.x < display.len() {
                    display.replace_range(self.state.cursor.x..=self.state.cursor.x, "_");
                } else {
                    display.push('_');
                }
                print!("{}", display);
            } else {
                print!("{}", line);
            }
        }

        execute!(
            stdout,
            cursor::MoveTo(0, self.state.buffer.len() as u16 + 1)
        )?;
        print!(
            "[{}] {:?} | Satır {}/{}",
            if self.state.dirty { "DEGISTI" } else { "KAYITLI" },
            self.state.filename,
            self.state.cursor.y + 1,
            self.state.buffer.len()
        );

        if self.mode == Mode::Command {
            execute!(
                stdout,
                cursor::MoveTo(0, self.state.buffer.len() as u16 + 2)
            )?;
            print!(":{}", self.command);
        }

        if self.confirm_exit {
            execute!(
                stdout,
                cursor::MoveTo(0, self.state.buffer.len() as u16 + 3)
            )?;
            print!("Kaydetmek ister misin? (y/n)");
        }

        if self.ask_filename {
            execute!(
                stdout,
                cursor::MoveTo(0, self.state.buffer.len() as u16 + 4)
            )?;
            print!("Dosya adi: {}", self.input_filename);
        }

        execute!(
            stdout,
            cursor::MoveTo(self.state.cursor.x as u16, self.state.cursor.y as u16)
        )?;
        stdout.flush()?;
        Ok(())
    }

    fn process_command(&mut self, stdout: &mut io::Stdout) -> Result<bool> {
        let cmd = self.command.trim().to_string();
        match cmd.as_str() {
            "w" => {
                if let Some(name) = self.state.filename.clone() {
                    let _ = self.save_to_file(name);
                } else {
                    self.ask_filename = true;
                }
            }
            "q" => {
                if self.state.dirty {
                    self.confirm_exit = true;
                    self.pending_save = true;
                } else {
                    return Ok(true);
                }
            }
            "wq" => {
                if let Some(name) = self.state.filename.clone() {
                    self.save_to_file(name)?;
                    return Ok(true);
                } else {
                    self.ask_filename = true;
                }
            }
            _ => {}
        }
        self.command.clear();
        self.mode = Mode::Insert;
        self.render(stdout)?;
        Ok(false)
    }

    fn matching_pair(c: char) -> Option<char> {
        match c {
            '(' => Some(')'),
            '{' => Some('}'),
            '[' => Some(']'),
            '"' | '\'' => Some(c),
            _ => None,
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(1).cloned();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let mut editor = Editor::new(filename);

    loop {
        editor.render(&mut stdout)?;

        match event::read()? {
            Event::Key(key) => {
                if editor.ask_filename {
                    match key.code {
                        KeyCode::Char(c) => editor.input_filename.push(c),
                        KeyCode::Backspace => {
                            editor.input_filename.pop();
                        }
                        KeyCode::Enter => {
                            let name = editor.input_filename.clone();
                            let _ = editor.save_to_file(name);
                            editor.ask_filename = false;
                        }
                        KeyCode::Esc => {
                            editor.ask_filename = false;
                            editor.pending_save = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                if editor.confirm_exit {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Some(name) = editor.state.filename.clone() {
                                let _ = editor.save_to_file(name);
                                break;
                            } else {
                                editor.ask_filename = true;
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => break,
                        KeyCode::Esc => {
                            editor.confirm_exit = false;
                            editor.pending_save = false;
                        }
                        _ => {}
                    }
                    continue;
                }

                match editor.mode {
                    Mode::Insert => match key.code {
                        KeyCode::Char(':') => editor.mode = Mode::Command,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            editor.copy_selection()
                        }
                        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            editor.paste()
                        }
                        KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            editor.undo()
                        }
                        KeyCode::Char(c) => editor.insert(c),
                        KeyCode::Backspace => editor.delete(),
                        KeyCode::Enter => editor.newline(),
                        KeyCode::Up => {
                            if editor.state.cursor.y > 0 {
                                editor.state.cursor.y -= 1;
                                editor.state.cursor.x =
                                    editor.state.cursor.x.min(editor.state.buffer[editor.state.cursor.y].len());
                            }
                        }
                        KeyCode::Down => {
                            if editor.state.cursor.y + 1 < editor.state.buffer.len() {
                                editor.state.cursor.y += 1;
                                editor.state.cursor.x =
                                    editor.state.cursor.x.min(editor.state.buffer[editor.state.cursor.y].len());
                            }
                        }
                        KeyCode::Left => {
                            if editor.state.cursor.x > 0 {
                                editor.state.cursor.x -= 1;
                            } else if editor.state.cursor.y > 0 {
                                editor.state.cursor.y -= 1;
                                editor.state.cursor.x =
                                    editor.state.buffer[editor.state.cursor.y].len();
                            }
                        }
                        KeyCode::Right => {
                            if editor.state.cursor.x < editor.state.buffer[editor.state.cursor.y].len() {
                                editor.state.cursor.x += 1;
                            } else if editor.state.cursor.y + 1 < editor.state.buffer.len() {
                                editor.state.cursor.y += 1;
                                editor.state.cursor.x = 0;
                            }
                        }
                        KeyCode::Esc => break,
                        _ => {}
                    },
                    Mode::Command => match key.code {
                        KeyCode::Char(c) => editor.command.push(c),
                        KeyCode::Backspace => {
                            editor.command.pop();
                        }
                        KeyCode::Enter => {
                            if editor.process_command(&mut stdout)? {
                                break;
                            }
                        }
                        KeyCode::Esc => {
                            editor.command.clear();
                            editor.mode = Mode::Insert;
                        }
                        _ => {}
                    },
                }
            }
            _ => {}
        }
    }

    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

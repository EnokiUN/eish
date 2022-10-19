use crossterm::{
    cursor::MoveLeft,
    event::{self, Event, KeyCode, KeyModifiers},
    style::Stylize,
    terminal::{disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use std::{
    env,
    error::Error,
    fmt::Display,
    io::{stdout, ErrorKind, Stdout, Write},
    path::PathBuf,
    process::Command,
};

#[derive(Debug)]
pub enum Input {
    Command(String),
    Exit,
}

#[derive(Debug)]
pub struct Shell {
    pub stdout: Stdout,
    pub path: PathBuf,
}

impl Shell {
    /// Handle input and return whether to exit or not
    pub fn handle_input(&mut self, input: Input) -> Result<bool, Box<dyn Error>> {
        match input {
            Input::Command(input) => {
                if input.is_empty() {
                    return Ok(false);
                }
                if input.starts_with("//") {
                    return Ok(false);
                }
                let input: Vec<&str> = input.trim().split(' ').collect();
                match input[0] {
                    "exit" => {
                        self.write("See you later, Bye!\r")?;
                        return Ok(false);
                    }
                    _ => {
                        let mut cmd = Command::new(input[0]);
                        if input.len() > 1 {
                            cmd.args(input[1..].iter());
                        }
                        match cmd.spawn() {
                            Ok(mut cmd) => {
                                disable_raw_mode()?;
                                cmd.wait()?;
                                enable_raw_mode()?;
                            }
                            Err(err) => match err.kind() {
                                ErrorKind::NotFound => {
                                    self.write("Unknown command")?;
                                }
                                _ => {
                                    self.write(
                                        format!("Error running command: {:#?}", err)
                                            .replace('\n', "\r\n")
                                            .red(),
                                    )?;
                                }
                            },
                        }
                    }
                }
                Ok(false)
            }
            Input::Exit => Ok(true),
        }
    }
    pub fn get_input(&mut self) -> Result<Input, Box<dyn Error>> {
        let mut input = String::new();
        let mut idx = 0;
        write!(
            self.stdout,
            "\r\x1b[2K{}-{} {}\r\n\x1b[2K{} {}",
            idx.to_string().blue(),
            input.len().to_string().red(),
            self.path.to_str().unwrap().green(),
            "~>".magenta(),
            input
        )?;
        self.stdout.flush()?;
        loop {
            write!(
                self.stdout,
                "\x1b[F\x1b[2K{}-{} {}\r\n\x1b[2K{} {}",
                idx.to_string().blue(),
                input.len().to_string().red(),
                self.path.to_str().unwrap().green(),
                "~>".magenta(),
                input
            )?;
            if !input.is_empty() {
                if input.len() > idx {
                    self.stdout.queue(MoveLeft((input.len() - idx) as u16))?;
                }
            }
            self.stdout.flush()?;
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char(chr) => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            match chr {
                                'd' if input.is_empty() => {
                                    return Ok(Input::Exit);
                                }
                                'c' => {
                                    input.drain(..);
                                    idx = 0;
                                    // newline
                                    self.write("")?;
                                }
                                _ => {}
                            }
                        } else {
                            input.insert(idx, chr);
                            idx += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // newline
                        self.write("")?;
                        break;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.remove(idx - 1);
                            idx -= 1;
                        }
                    }
                    KeyCode::Left => {
                        if idx != 0 {
                            idx -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if idx < input.len() {
                            idx += 1;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        write!(
            self.stdout,
            "\x1b[2F\x1b[2K{} {}\r\n\x1b[2K",
            "~>".magenta(),
            input
        )?;
        self.stdout.flush()?;
        Ok(Input::Command(input))
    }
    pub fn write(&mut self, input: impl Display) -> Result<(), Box<dyn Error>> {
        writeln!(self.stdout, "{}\r", input)?;
        self.stdout.flush()?;
        Ok(())
    }
}

fn main() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        disable_raw_mode().unwrap();
        hook(p);
    }));
    enable_raw_mode().unwrap();
    let mut sh = Shell {
        stdout: stdout(),
        path: env::current_dir().unwrap_or_else(|_| env::var("HOME").unwrap().parse().unwrap()),
    };
    sh.write("Welcome to EISH").unwrap();
    while let Ok(input) = sh.get_input() {
        if sh.handle_input(input).unwrap() == true {
            break;
        }
    }
    disable_raw_mode().unwrap();
}

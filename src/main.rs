use rand::{
    distributions::{Alphanumeric, DistString},
    Rng,
};
use std::{
    error::Error,
    io::{stdin, stdout, StdoutLock, Write},
};
use termion::{
    clear, color, cursor,
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
    screen::{AlternateScreen, IntoAlternateScreen},
    style,
};

const HEADER_COLOR: color::Fg<color::LightGreen> = color::Fg(color::LightGreen);
const TITLE_COLOR: color::Fg<color::White> = color::Fg(color::White);
const LIST_COLOR: color::Fg<color::LightBlack> = color::Fg(color::LightBlack);
const POINTER_COLOR: color::Fg<color::White> = color::Fg(color::White);
const FOOTER_COLOR: color::Fg<color::LightBlue> = color::Fg(color::LightBlue);

#[derive(Debug, Clone, Copy)]
enum Direction {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy)]
struct Positions {
    header: (u16, u16),
    title: (u16, u16),
    list_top: (u16, u16),
    footer: (u16, u16),
}

impl Positions {
    fn new(list: (u16, u16), border: (u16, u16)) -> Self {
        // TODO: if list.0 > (terminal.0 - border.0) => wrap

        Self {
            header: (border.0, border.1),
            title: (border.0, border.1 + 3),
            list_top: (border.0, border.1 + 5),
            footer: (border.0, border.0 + list.1 + 2),
        }
    }
}

#[derive(Debug, Clone)]
struct Interface {
    pointer: (u16, u16),
    data: Vec<String>,
    list_len: usize,
    index: usize,
    pos: Positions,
}

impl Interface {
    pub fn new(data: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let list_len = data.len();
        let list_w = data
            .iter()
            .max_by(|x, y| x.len().cmp(&y.len()))
            .unwrap()
            .len();
        let positions = Positions::new((list_w as u16, data.len() as u16), (6, 2));

        Ok(Self {
            pointer: positions.list_top,
            data,
            list_len,
            index: 0,
            pos: positions,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let stdin = stdin();
        let mut stdout = stdout()
            .lock()
            .into_raw_mode()
            .unwrap()
            .into_alternate_screen()
            .unwrap();

        self.clear(&mut stdout)?;

        // header
        write!(
            stdout,
            "{}{}{}Connected to the server at 123.1.2.3:8080{}",
            cursor::Goto(self.pos.header.0, self.pos.header.1),
            style::Bold,
            HEADER_COLOR,
            style::Reset
        )?;

        // title
        write!(
            stdout,
            "{}{}{}Available files:{}",
            cursor::Goto(self.pos.title.0, self.pos.title.1),
            style::Bold,
            TITLE_COLOR,
            style::Reset
        )?;

        // list
        self.write_list(&mut stdout)?;

        // footer (TODO: progress bar)
        write!(
            stdout,
            "{}{}{}Downloading: {}, {} & {}...{}",
            cursor::Goto(self.pos.footer.0, self.pos.footer.1),
            style::Bold,
            FOOTER_COLOR,
            self.data[0],
            self.data[1],
            self.data[2],
            style::Reset
        )?;

        stdout.flush()?;

        for c in stdin.keys() {
            match c? {
                Key::Char('q') => break,
                Key::Char('j') => {
                    if self.update_pointer(Direction::Down) {
                        self.write_pointer(&mut stdout)?;
                        self.clear_pointer(&mut stdout, Direction::Down)?;
                    }
                }
                Key::Char('k') => {
                    if self.update_pointer(Direction::Up) {
                        self.write_pointer(&mut stdout)?;
                        self.clear_pointer(&mut stdout, Direction::Up)?;
                    }
                }
                Key::Char('\n') => {
                    todo!("gray out the current file")
                }
                _ => {}
            };
        }

        write!(stdout, "{}", cursor::Show).unwrap();

        Ok(())
    }

    fn clear(
        &self,
        stdout: &mut AlternateScreen<RawTerminal<StdoutLock>>,
    ) -> Result<(), Box<dyn Error>> {
        write!(stdout, "{}{}", clear::All, cursor::Hide)?;

        Ok(())
    }

    fn write_list(
        &self,
        stdout: &mut AlternateScreen<RawTerminal<StdoutLock>>,
    ) -> Result<(), Box<dyn Error>> {
        let this_data = self.data.clone();
        let list_pos = self.pos.list_top;

        this_data.iter().enumerate().for_each(|(i, item)| {
            write!(
                stdout,
                "{}{}{}{}",
                cursor::Goto(list_pos.0, list_pos.1 + i as u16),
                LIST_COLOR,
                item,
                style::Reset
            )
            .unwrap();
        });

        write!(stdout, "{}", cursor::Goto(list_pos.0, list_pos.1))?;

        Ok(())
    }

    fn clear_pointer(
        &self,
        stdout: &mut AlternateScreen<RawTerminal<StdoutLock>>,
        direction: Direction,
    ) -> Result<(), Box<dyn Error>> {
        let (pos, item) = match direction {
            Direction::Up => (
                (self.pointer.0, self.pointer.1 + 1),
                self.data[self.index + 1].clone(),
            ),
            Direction::Down => (
                (self.pointer.0, self.pointer.1 - 1),
                self.data[self.index - 1].clone(),
            ),
        };

        write!(
            stdout,
            "{}{}{}{}{}",
            cursor::Goto(pos.0, pos.1),
            clear::CurrentLine,
            LIST_COLOR,
            item,
            style::Reset,
        )?;

        stdout.flush()?;

        Ok(())
    }

    fn write_pointer(
        &self,
        stdout: &mut AlternateScreen<RawTerminal<StdoutLock>>,
    ) -> Result<(), Box<dyn Error>> {
        write!(
            stdout,
            "{}{}{}{} * {}{}",
            cursor::Goto(self.pointer.0, self.pointer.1),
            clear::CurrentLine,
            style::Bold,
            POINTER_COLOR,
            self.data[self.index],
            style::Reset
        )?;

        stdout.flush()?;

        Ok(())
    }

    fn update_pointer(&mut self, direction: Direction) -> bool {
        match direction {
            Direction::Up => {
                if self.index > 0 && self.index <= self.list_len {
                    self.pointer.1 -= 1;
                    self.index -= 1;

                    return true;
                }
            }
            Direction::Down => {
                if self.index < self.list_len - 1 {
                    self.pointer.1 += 1;
                    self.index += 1;

                    return true;
                }
            }
        }

        false
    }
}

fn rand_string() -> String {
    let len = rand::thread_rng().gen_range(5..30);
    Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}

fn main() {
    let mut data = Vec::new();
    (0..15).into_iter().for_each(|_| data.push(rand_string()));

    let mut interface = Interface::new(data).unwrap();
    interface.run().unwrap();
}

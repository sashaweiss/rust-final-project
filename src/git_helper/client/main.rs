/// Much of this example borrows from the `tui-rs` examples, and was modified for our purposes.
/// See: https://github.com/fdehau/tui-rs/blob/master/examples/user_input.rs
extern crate chrono;
extern crate syncterm;
extern crate termion;
extern crate tui;

use chrono::prelude::*;
const TIME_FORMAT: &'static str = "%H:%M:%S";

use syncterm::Key;
use syncterm::client::*;
use syncterm::messages::*;

use tui::Terminal;
use tui::backend::MouseBackend;
use tui::layout::{Direction, Group, Rect, Size};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Widget};

struct App {
    size: Rect,
    input: String,
    input_mode: Mode,
    messages: Vec<(DateTime<Local>, String, String)>,
    commands: Vec<(DateTime<Local>, String, String, String)>,
    terminal: Terminal<MouseBackend>,
}

impl App {
    fn new() -> App {
        App {
            size: Rect::default(),
            input: String::new(),
            input_mode: Mode::Chat,
            messages: Vec::new(),
            commands: Vec::new(),
            terminal: Terminal::new(MouseBackend::new().unwrap()).unwrap(),
        }
    }
}

impl ShellClient for App {
    fn on_key(&mut self, key: syncterm::Key) -> KeyAction {
        match key {
            Key::Ctrl('c') | Key::Esc => {
                return KeyAction::Exit;
            }
            Key::Char('\n') => {
                let message = self.input.drain(..).collect::<String>();
                match message.as_ref() {
                    "CHAT" => {
                        self.input_mode = Mode::Chat;
                    }
                    "CMD" => {
                        self.input_mode = Mode::Cmd;
                    }
                    "CLEAR" => {
                        match self.input_mode {
                            Mode::Cmd => self.commands.clear(),
                            Mode::Chat => self.messages.clear(),
                        };
                    }
                    _ => {
                        return KeyAction::SendMessage(Message {
                            content: message,
                            mode: Mode::Chat,
                            user_name: "jennie".to_owned(),
                        });
                    }
                }
            }
            Key::Char(c) => {
                self.input.push(c);
            }
            Key::Backspace => {
                self.input.pop();
            }
            _ => {}
        }

        KeyAction::DoNothing
    }

    fn receive_response(&mut self, response: Response) {
        match response.og_msg.mode {
            Mode::Chat => {
                self.messages
                    .push((Local::now(), response.og_msg.user_name, response.response));
            }
            Mode::Cmd => {
                self.commands.push((
                    Local::now(),
                    response.og_msg.user_name,
                    response.og_msg.content,
                    response.response,
                ));
            }
        };
    }

    fn first_draw(&mut self) {
        self.terminal.clear().unwrap();
        self.terminal.hide_cursor().unwrap();
        self.size = self.terminal.size().unwrap();

        self.draw();
    }

    fn last_draw(&mut self) {
        self.terminal.show_cursor().unwrap();
    }

    fn draw(&mut self) {
        let mut size = self.terminal.size().unwrap();
        self.terminal.resize(size).unwrap();

        size = self.terminal.size().unwrap();
        let mode = &self.input_mode;
        let input = &self.input;
        let messages = &self.messages;
        let commands = &self.commands;

        Group::default()
            .direction(Direction::Vertical)
            .margin(2)
            .sizes(&[Size::Fixed(3), Size::Min(1)])
            .render(&mut self.terminal, &size, |t, chunks| {
                Paragraph::default()
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL).title(match *mode {
                        Mode::Chat => "Chat",
                        Mode::Cmd => "Command",
                    }))
                    .text(input)
                    .render(t, &chunks[0]);

                Group::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .sizes(&[Size::Percent(50), Size::Percent(50)])
                    .render(t, &chunks[1], |t, chunks| {
                        // Use Paragraphs so we can get text wrapping
                        let messages: String = messages.iter().rev().fold(
                            "".to_owned(),
                            |mut acc, (t, u, m)| {
                                acc.push_str(&format!(
                                    "{}: {}: {}\n",
                                    t.format(TIME_FORMAT).to_string(),
                                    u,
                                    m
                                ));
                                acc
                            },
                        );
                        Paragraph::default()
                            .block(Block::default().borders(Borders::ALL).title("Messages"))
                            .wrap(true)
                            .text(&messages)
                            .render(t, &chunks[0]);

                        let commands: String = commands.iter().rev().fold(
                            "".to_owned(),
                            |mut acc, (t, u, c, m)| {
                                acc.push_str(&format!(
                                    "{}: {} >> {}\n{}{}\n",
                                    t.format(TIME_FORMAT).to_string(),
                                    u,
                                    c,
                                    m,
                                    if m.ends_with("\n") { "" } else { "\n" }
                                ));
                                acc
                            },
                        );
                        Paragraph::default()
                            .block(Block::default().borders(Borders::ALL).title("Commands"))
                            .wrap(true)
                            .text(&commands)
                            .render(t, &chunks[1]);
                    });
            });

        self.terminal.draw().unwrap();
    }
}

fn main() {
    connect(App::new());
}

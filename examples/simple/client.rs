/// Much of this example borrows from the `tui-rs` examples, and was modified for our purposes.
/// See: https://github.com/fdehau/tui-rs/blob/master/examples/user_input.rs
use chrono::prelude::*;
use syncterm;

use messages::*;

pub struct App {
    user_name: String,
    input_buffer: String,
    input_mode: Mode,
    messages: Vec<(DateTime<Local>, String, String)>,
}

impl App {
    pub fn new(user_name: String) -> App {
        App {
            user_name,
            input_buffer: String::new(),
            input_mode: Mode::Lower,
            messages: Vec::new(),
        }
    }
}

impl syncterm::client::ShellClient<Message, Response> for App {
    fn server_url(&self) -> String {
        "127.0.0.1:8080".to_owned()
    }

    fn on_key(&mut self, key: syncterm::client::Key) -> syncterm::client::KeyAction<Message> {
        match key {
            syncterm::client::Key::Ctrl('c') | syncterm::client::Key::Esc => {
                return syncterm::client::KeyAction::Exit;
            }
            syncterm::client::Key::Char('\n') => {
                let message = self.input_buffer.drain(..).collect::<String>();
                match message.as_ref() {
                    "LOWER" => {
                        self.input_mode = Mode::Lower;
                    }
                    "UPPER" => {
                        self.input_mode = Mode::Upper;
                    }
                    _ => {
                        return syncterm::client::KeyAction::SendMessage(Message {
                            content: message,
                            mode: self.input_mode.clone(),
                            user_name: self.user_name.clone(),
                        });
                    }
                }
            }
            syncterm::client::Key::Char(c) => {
                self.input_buffer.push(c);
            }
            syncterm::client::Key::Backspace => {
                self.input_buffer.pop();
            }
            _ => {}
        }

        syncterm::client::KeyAction::DoNothing
    }

    fn receive_response(&mut self, response: Response) {
        self.messages
            .push((Local::now(), response.og_msg.user_name, response.response));
    }

    fn first_draw(&mut self) {
        println!("Welcome! Type UPPER for uppercase mode and LOWER for lowercase mode");
    }

    fn last_draw(&mut self) {
        println!("GOODBYE!!!");
    }

    fn draw(&mut self) {
        if let Some(m) = self.messages.pop() {
            println!("{}: {} >> {}", m.0.format("%H:%M:%S").to_string(), m.1, m.2);
        }
    }
}

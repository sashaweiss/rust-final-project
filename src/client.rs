use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use Key;
use messages::*;
use ui;

use rand::random;
use serde_json;

pub struct ShellConnection {
    stream: TcpStream,
    remote_url: String,
}

impl ShellConnection {
    pub fn connect(url: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(url)?;

        Ok(Self {
            stream,
            remote_url: url.to_owned(),
        })
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        let stream_clone = self.stream.try_clone()?;

        Ok(Self {
            stream: stream_clone,
            remote_url: self.remote_url.clone(),
        })
    }

    pub fn send_input(&mut self, content: &str, mode: &Mode, user_name: &str) -> io::Result<usize> {
        let input = Message {
            content: content.to_owned(),
            mode: mode.clone(),
            user_name: user_name.to_owned(),
        };

        let mut sendable = serde_json::to_vec(&input).unwrap();
        sendable.push(b'\n');

        self.stream.write(&sendable)
    }

    pub fn read_response(&self) -> Result<Response, String> {
        let mut resp = String::new();
        BufReader::new(&self.stream)
            .read_line(&mut resp)
            .map_err(|e| format!("Error reading: {:?}", e))?;

        serde_json::from_str(&resp).map_err(|e| format!("Error reading: {:?}", e))
    }
}

pub struct ShellClient {
    pub(crate) user_name: String,
    pub(crate) chat: String,
    pub(crate) cmd: String,
    pub(crate) clear: String,
    pub(crate) break_on: Key,
    pub(crate) input_on_top: bool,
    pub(crate) chat_on_left: bool,
}

impl ShellClient {
    pub fn default() -> Self {
        ShellClient {
            user_name: (0..4).map(|_| random::<char>()).collect(),
            chat: "CHAT".to_owned(),
            cmd: "CMD".to_owned(),
            clear: "CLEAR".to_owned(),
            break_on: Key::Ctrl('c'),
            input_on_top: true,
            chat_on_left: true,
        }
    }

    pub fn user_name(&mut self, name: &str) -> &mut Self {
        self.user_name = name.to_owned();
        self
    }

    pub fn chat(&mut self, word: &str) -> &mut Self {
        self.chat = word.to_owned();
        self
    }

    pub fn cmd(&mut self, word: &str) -> &mut Self {
        self.cmd = word.to_owned();
        self
    }

    pub fn clear(&mut self, word: &str) -> &mut Self {
        self.clear = word.to_owned();
        self
    }

    pub fn break_on(&mut self, key: Key) -> &mut Self {
        self.break_on = key;
        self
    }

    pub fn input_on_top(&mut self, yeah: bool) -> &mut Self {
        self.input_on_top = yeah;
        self
    }

    pub fn chat_on_left(&mut self, yeah: bool) -> &mut Self {
        self.chat_on_left = yeah;
        self
    }

    pub fn render(&self, connection: &mut ShellConnection) {
        ui::render(connection, self);
    }
}

pub fn connect_and_echo() {
    let mut args = ::std::env::args(); // TODO: make this safer
    args.next();
    let user_name = match args.next() {
        Some(n) => n,
        None => (0..4).map(|_| random::<char>()).collect(),
    };

    let mut connection = ShellConnection::connect("127.0.0.1:8080").unwrap();

    ui::render(&mut connection, &ShellClient::default());
}

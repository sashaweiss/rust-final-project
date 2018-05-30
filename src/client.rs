use std::io::{stdin, stdout, BufRead, BufReader, Read, Result, Write};
use std::net::TcpStream;
use std::thread;

use command::*;
use ui;

use rand::random;
use serde_json;

pub struct ShellConnection<S: Read + Write> {
    stream: S,
    remote_url: String,
}

impl ShellConnection<TcpStream> {
    pub fn connect(url: &str) -> Result<Self> {
        let stream = TcpStream::connect(url)?;

        Ok(Self {
            stream,
            remote_url: url.to_owned(),
        })
    }

    pub fn try_clone(&self) -> Result<Self> {
        let stream_clone = self.stream.try_clone()?;

        Ok(Self {
            stream: stream_clone,
            remote_url: self.remote_url.clone(),
        })
    }

    pub fn send_input(&mut self, content: &str, mode: &Mode, user_name: &str) -> Result<usize> {
        let input = Message {
            content: content.to_owned().into_bytes(),
            mode: mode.clone(),
            user_name: user_name.to_owned(),
        };

        let mut sendable = serde_json::to_vec(&input).unwrap();
        sendable.push(b'\n');

        self.stream.write(&sendable)
    }

    pub fn read_response(&self) -> Message {
        let mut resp = String::new();
        BufReader::new(&self.stream).read_line(&mut resp).unwrap();

        serde_json::from_str(&resp).unwrap()
    }
}

pub fn connect_and_echo() {
    let mut args = ::std::env::args(); //TODO: make this safer
    args.next();
    let user_name = match args.next() {
        Some(n) => n,
        None => (0..4).map(|_| random::<char>()).collect(),
    };

    let mut connection = ShellConnection::connect("127.0.0.1:8080").unwrap();

    ui::render(&mut connection, &user_name);
}

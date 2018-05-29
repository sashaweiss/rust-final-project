use std::io::{stdin, stdout, BufRead, BufReader, Read, Result, Write};
use std::net::TcpStream;
use std::thread;

use command::*;

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

    pub fn send_command(&mut self, cmd: &str) -> Result<usize> {
        let n_bytes = self.stream.write(cmd.as_bytes())?;
        Ok(n_bytes + self.stream.write(&[b'\n'])?)
    }

    pub fn read_response(&self) -> CommandResponse {
        let mut resp = String::new();
        BufReader::new(&self.stream).read_line(&mut resp).unwrap();

        serde_json::from_str(&resp).unwrap()
    }
}

pub fn connect_and_echo() {
    let mut connection = ShellConnection::connect("127.0.0.1:8080").unwrap();
    let read_connection = connection.try_clone().unwrap();

    thread::spawn(move || {
        let mut lines = BufReader::new(stdin()).lines();

        while let Some(Ok(line)) = lines.next() {
            connection.send_command(&line).unwrap();
        }
    });

    loop {
        let resp = read_connection.read_response();

        stdout().write_all(&resp.stdout).unwrap();
    }
}

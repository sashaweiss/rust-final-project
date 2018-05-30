use std::io::{stdout, BufRead, BufReader, Read, Result, Write};
use std::net::TcpStream;
use std::thread;
extern crate rustyline;

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

    pub fn send_input(&mut self, content: &str, mode: &Mode) -> Result<usize> {

        let input = UserInput {
            content: content.to_owned().into_bytes(),
            mode: mode.clone(),
        };

        let mut sendable = serde_json::to_vec(&input).unwrap();
        sendable.push(b'\n');

        let n_bytes = self.stream.write(&sendable)?;
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

        let mut mode = Mode::Chat;

        loop {

            let mut rl = rustyline::Editor::<()>::new();
            let read_line = rl.readline(&mode.prompt());

            match read_line {
                Ok(line) => {
                    match line.as_ref() {
                        "EXIT" => {
                            println!("Exiting shared terminal");
                            break;
                        },
                        "CHAT" => {
                            mode = Mode::Chat;
                            println!("Switched to Chat mode");

                        },
                        "CMD" => {
                            mode = Mode::Cmd;
                            println!("Switched to Cmd mode");

                        }
                        _ => {
                            connection.send_input(&line, &mode).unwrap();
                        }
                    }
                },
                Err(err) => {
                    println!("Error: {:?}, exiting shared terminal", err);
                    break;
                }
            }

        }

    });

    loop {
        let resp = read_connection.read_response();

        stdout().write_all(&resp.stdout).unwrap();
    }
}


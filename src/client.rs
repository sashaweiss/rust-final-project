use std::io::{stdin, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;
extern crate rustyline;

use self::rustyline::Editor;
use self::rustyline::error::ReadlineError;

pub fn connect_and_echo() {
    let mut writestream = TcpStream::connect("127.0.0.1:8080").unwrap();
    let readstream = BufReader::new(writestream.try_clone().expect("Failed to clone stream"));

    thread::spawn(move || {

        let mut mode = Mode::Chat;

        loop {

            let mut rl = rustyline::Editor::<()>::new();
            let readline = rl.readline(mode.prompt());
            let mut new_line;
            match readline {
                Ok(mut line) => {
                    if line == "EXIT" {
                        println!("Exiting shared terminal");
                        break;
                    }
                    line.push('\n');

                    writestream
                        .write(input)
                        .expect("Failed to stream echo");

                },
                Err(err) => {
                    println!("Error: {:?}, exiting shared terminal", err);
                    break;
                }
            }

        }

    });

    let mut lines = readstream.lines();
    while let Some(Ok(response)) = lines.next() {
        println!("response: {}", response);
    }
}


pub enum UserInput {
    Chat(String),
    Command(String),
}

pub enum Mode {
    Chat,
    Command,

}

impl Status {
    pub fn key_word(&self) -> String {
        use self::Status::*;
        match *self {
            Chat => "CHAT".to_string(),
           Command => "COMMAND".to_string(),
        }
    },
    pub fn prompt(&self) -> String {
        use self::Status::*;
        match *self {
            Chat => "CHAT >>".to_string(),
            Command => ">>".to_string(),
        }
    }
}


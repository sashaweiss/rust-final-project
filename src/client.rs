use std::io::{stdin, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;

pub fn connect_and_echo() {
    let mut writestream = TcpStream::connect("127.0.0.1:8080").unwrap();
    let readstream = BufReader::new(writestream.try_clone().expect("Failed to clone stream"));

    thread::spawn(move || {
        let mut lines = BufReader::new(stdin()).lines();

        while let Some(Ok(mut line)) = lines.next() {
            line.push('\n');

            writestream
                .write(line.as_bytes())
                .expect("Failed to stream echo");
        }
    });

    let mut lines = readstream.lines();
    while let Some(Ok(response)) = lines.next() {
        println!("response: {}", response);
    }
}

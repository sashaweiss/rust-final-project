use std::io::Write;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;

pub fn connect_and_echo() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();

    stream
        .write("echo hello world\n".as_bytes())
        .expect("Failed to stream echo");

    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .expect("Failed to read line");
    println!("response: {}", response);
}

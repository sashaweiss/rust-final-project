use std::io::Write;
use std::net::TcpStream;

pub fn connect_and_echo() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();

    stream.write("echo hello world\n".as_bytes());
}

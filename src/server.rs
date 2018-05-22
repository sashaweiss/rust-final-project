use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};

pub fn spawn_bash_and_listen() {
    let bash_child = Command::new("bash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start bash");

    let mut bash_out = BufReader::new(bash_child.stdout.expect("failed to get bash stdout"));
    let mut bash_in = bash_child.stdin.expect("failed to get stdin: {}");

    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let (stream, _) = listener.accept().unwrap();
    let mut bufstream = BufReader::new(stream);

    let mut input = String::new();
    bufstream.read_line(&mut input);
    bash_in.write(input.as_bytes());

    let mut output = String::new();
    bash_out.read_line(&mut output);
    println!("output: {}", output);
}

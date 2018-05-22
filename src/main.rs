use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};

fn main() {
    let bash_child = Command::new("bash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start bash");

    let mut bash_out = BufReader::new(bash_child.stdout.expect("failed to get bash stdout"));
    let mut bash_in = bash_child.stdin.expect("failed to get stdin: {}");

    bash_in.write("echo hello world\n".as_bytes());
    let mut output = String::new();
    bash_out.read_line(&mut output);

    println!("output: {}", output);
}

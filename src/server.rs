use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;

use chan;

pub fn spawn_bash_and_listen() {
    let bash_child = Command::new("bash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start bash");

    let mut bash_out = BufReader::new(bash_child.stdout.expect("failed to get bash stdout"));
    let mut bash_in = bash_child.stdin.expect("failed to get stdin: {}");

    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let (incoming_sx, incoming_rx) = chan::sync::<String>(0);
    let (outgoing_sx, outgoing_rx) = chan::sync::<String>(0);

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let sx_thread = incoming_sx.clone();
                    let rx_thread = outgoing_rx.clone();

                    let stream_reader =
                        BufReader::new(stream.try_clone().expect("Failed to clone stream"));
                    thread::spawn(move || {
                        let mut lines = stream_reader.lines();

                        while let Some(Ok(mut line)) = lines.next() {
                            line.push('\n');
                            sx_thread.send(line);
                        }
                    });

                    thread::spawn(move || loop {
                        let output = rx_thread.recv().expect("Nothing to receive");
                        stream.write(output.as_bytes()).expect("Failed to write");
                    });
                }
                Err(e) => {
                    panic!("Oh no: {}", e);
                }
            }
        }
    });

    loop {
        let input = incoming_rx.recv().expect("Nothing to receive");
        println!("Received input: {:?}, writing to Bash...", input);
        bash_in
            .write(input.as_bytes())
            .expect("Failed to write input");

        print!("Reading line from Bash...");
        let mut output = String::new();
        bash_out
            .read_line(&mut output)
            .expect("Failed to read line");
        println!(" {:?}. Sending back...", output);

        outgoing_sx.send(output);
    }
}

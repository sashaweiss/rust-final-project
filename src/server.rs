use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

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

    let (incoming_sx, incoming_rx) = channel::<String>();

    let outgoing_sxs = Arc::new(Mutex::new(Vec::new()));
    // let (outgoing_sx, outgoing_rx) = channel::<String>();

    let outgoing_sxs_thread = outgoing_sxs.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let incoming_sx_stream = incoming_sx.clone();
                    let outgoing_sxs_stream = outgoing_sxs_thread.clone();
                    thread::spawn(move || {
                        // TODO: lock some item to prevent sending/receiving while threads spin up

                        println!(
                            "Received connection from {}!",
                            stream.peer_addr().expect("Failed to get stream remote IP")
                        );

                        let (outgoing_sx_stream, outgoing_rx_stream) = channel::<String>();
                        {
                            outgoing_sxs_stream
                                .lock()
                                .expect("Poisoned Vec of outgoing sxs")
                                .push(outgoing_sx_stream);
                        }

                        let read_stream =
                            BufReader::new(stream.try_clone().expect("Failed to clone stream"));
                        let read_handle = thread::spawn(move || {
                            println!(
                                "Thread {:?} starting up to read stream",
                                thread::current().id()
                            );

                            let mut lines = read_stream.lines();
                            while let Some(maybe_line) = lines.next() {
                                match maybe_line {
                                    Ok(mut line) => {
                                        line.push('\n');
                                        println!(
                                            "Sending line {} from thread {:?}",
                                            line,
                                            thread::current().id()
                                        );
                                        incoming_sx_stream.send(line);
                                    }
                                    Err(e) => {
                                        panic!(
                                            "Stream in thread {:?} failed to read with error {}",
                                            thread::current().id(),
                                            e
                                        );
                                    }
                                }
                            }

                            println!(
                                "Thread {:?} read None from stream, shutting down...",
                                thread::current().id()
                            );
                        });

                        let write_handle = thread::spawn(move || {
                            println!(
                                "Thread {:?} starting to read response lines",
                                thread::current().id()
                            );

                            while let Ok(output) = outgoing_rx_stream.recv() {
                                println!(
                                    "Thread {:?} received response line {:?}",
                                    thread::current().id(),
                                    output
                                );
                                if let Err(e) = stream.write(output.as_bytes()) {
                                    panic!(
                                        "Stream in thread {:?} failed to write with error {}",
                                        thread::current().id(),
                                        e
                                    );
                                }
                            }

                            println!(
                                "Thread {:?} receiver closed, shutting down...",
                                thread::current().id()
                            );
                        });

                        // TODO: Unlock read/write signal

                        if let Err(e) = read_handle.join() {
                            println!("Reader thread panicked with message {:?}", e);
                        };
                        if let Err(e) = write_handle.join() {
                            println!("Reader thread panicked with message {:?}", e);
                        };
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

        let mut guard = outgoing_sxs.lock().expect("Poisoned Vec of outgoing sxs");
        println!(
            "Received line {:?}, attempting to send to {} clients",
            output,
            guard.len()
        );
        guard.retain(|outgoing_sx| outgoing_sx.send(output.clone()).is_ok());
        println!("{} clients successfully sent to", guard.len());
    }
}
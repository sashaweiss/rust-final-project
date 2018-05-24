use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
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

    let (stream_to_shell_sx, stream_to_shell_rx) = channel::<String>();

    let shell_to_stream_sxs = Arc::new(Mutex::new(Vec::new()));

    {
        let shell_to_stream_sxs = shell_to_stream_sxs.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let sx = stream_to_shell_sx.clone();
                        let sxs = shell_to_stream_sxs.clone();
                        thread::spawn(move || {
                            handle_new_stream(sx, sxs, stream);
                        });
                    }
                    Err(e) => {
                        panic!("Oh no: {}", e);
                    }
                }
            }
        });
    }

    loop {
        let input = stream_to_shell_rx.recv().expect("Nothing to receive");
        println!("Received input: {:?}, writing to Bash...", input);
        bash_in
            .write(input.as_bytes())
            .expect("Failed to write input");

        print!("Reading line from Bash...");
        let mut output = String::new();
        bash_out
            .read_line(&mut output)
            .expect("Failed to read line");

        let mut guard = shell_to_stream_sxs
            .lock()
            .expect("Poisoned Vec of outgoing sxs");
        println!(
            "Received line {:?}, attempting to send to {} clients",
            output,
            guard.len()
        );
        guard.retain(|shell_to_stream_sx| shell_to_stream_sx.send(output.clone()).is_ok());
        println!("{} clients successfully sent to", guard.len());
    }
}

fn handle_new_stream(
    stream_to_shell_sx: Sender<String>,
    shell_to_stream_sxs: Arc<Mutex<Vec<Sender<String>>>>,
    stream: TcpStream,
) {
    // TODO: lock some item to prevent sending/receiving while threads spin up

    println!(
        "Received connection from {}!",
        stream.peer_addr().expect("Failed to get stream remote IP")
    );

    let read_stream = stream.try_clone().unwrap();
    let receive_handle = thread::spawn(move || {
        receive_and_pass_along_line(read_stream, stream_to_shell_sx);
    });

    let (shell_to_stream_sx, shell_to_stream_rx) = channel::<String>();
    {
        shell_to_stream_sxs.lock().unwrap().push(shell_to_stream_sx);
    }
    let response_handle = thread::spawn(move || relay_response_back(stream, shell_to_stream_rx));

    // TODO: Unlock read/write signal

    if let Err(e) = receive_handle.join() {
        println!("Command receiver thread panicked with message {:?}", e);
    };
    if let Err(e) = response_handle.join() {
        println!("Response relayer thread panicked with message {:?}", e);
    };
}

fn receive_and_pass_along_line(stream: TcpStream, stream_to_shell: Sender<String>) {
    println!(
        "Thread {:?} starting up to read stream",
        thread::current().id()
    );

    let mut lines = BufReader::new(&stream).lines();
    while let Some(maybe_line) = lines.next() {
        match maybe_line {
            Ok(mut line) => {
                line.push('\n');
                println!(
                    "Sending line {} from thread {:?}",
                    line,
                    thread::current().id()
                );
                stream_to_shell.send(line).unwrap();
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
}

fn relay_response_back(mut stream: TcpStream, shell_to_stream_rx: Receiver<String>) {
    println!(
        "Thread {:?} starting to read response lines",
        thread::current().id()
    );

    while let Ok(output) = shell_to_stream_rx.recv() {
        println!(
            "Thread {:?} received response line {:?}",
            thread::current().id(),
            output
        );

        match stream.write(output.as_bytes()) {
            Err(e) => {
                panic!(
                    "Stream in thread {:?} failed to write with error {}",
                    thread::current().id(),
                    e
                );
            }
            Ok(_e) => {}
        }
    }

    println!(
        "Thread {:?} receiver closed, shutting down...",
        thread::current().id()
    );
}

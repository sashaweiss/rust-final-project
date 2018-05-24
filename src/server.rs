use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
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

    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let (stm_shl_sx, stm_shl_rx) = channel::<String>();
    let shl_stm_sxs = Arc::new(Mutex::new(Vec::new()));

    let sxs = shl_stm_sxs.clone();
    thread::spawn(move || {
        handle_incoming_streams(sxs, listener, stm_shl_sx);
    });

    let mut bash_out = BufReader::new(bash_child.stdout.expect("failed to get bash stdout"));
    let mut bash_in = bash_child.stdin.expect("failed to get stdin: {}");
    loop {
        pipe_stream_to_shell_and_relay_response(
            &stm_shl_rx,
            &shl_stm_sxs,
            &mut bash_in,
            &mut bash_out,
        );
    }
}

fn pipe_stream_to_shell_and_relay_response(
    stm_shl_rx: &Receiver<String>,
    shl_stm_sxs: &Arc<Mutex<Vec<Sender<String>>>>,
    bash_in: &mut ChildStdin,
    bash_out: &mut BufReader<ChildStdout>,
) {
    let input = stm_shl_rx.recv().expect("Nothing to receive");
    println!("Received input: {:?}, writing to Bash...", input);
    bash_in
        .write(input.as_bytes())
        .expect("Failed to write input");

    print!("Reading line from Bash...");
    let mut output = String::new();
    bash_out
        .read_line(&mut output)
        .expect("Failed to read line");

    let mut guard = shl_stm_sxs.lock().expect("Poisoned Vec of outgoing sxs");
    println!(
        "Received line {:?}, attempting to send to {} clients",
        output,
        guard.len()
    );
    guard.retain(|shl_stm_sx| shl_stm_sx.send(output.clone()).is_ok());
    println!("{} clients successfully sent to", guard.len());
}

fn handle_incoming_streams(
    shl_stm_sxs: Arc<Mutex<Vec<Sender<String>>>>,
    listener: TcpListener,
    stm_shl_sx: Sender<String>,
) {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let sx = stm_shl_sx.clone();
                let sxs = shl_stm_sxs.clone();
                thread::spawn(move || {
                    handle_new_stream(sx, sxs, stream);
                });
            }
            Err(e) => {
                panic!("Oh no: {}", e);
            }
        }
    }
}

fn handle_new_stream(
    stm_shl_sx: Sender<String>,
    shl_stm_sxs: Arc<Mutex<Vec<Sender<String>>>>,
    stream: TcpStream,
) {
    // TODO: lock some item to prevent sending/receiving while threads spin up

    println!(
        "Received connection from {}!",
        stream.peer_addr().expect("Failed to get stream remote IP")
    );

    let read_stream = stream.try_clone().unwrap();
    let receive_handle = thread::spawn(move || {
        receive_and_pass_along_line(read_stream, stm_shl_sx);
    });

    let (shl_stm_sx, shl_stm_rx) = channel::<String>();
    {
        shl_stm_sxs.lock().unwrap().push(shl_stm_sx);
    }
    let response_handle = thread::spawn(move || relay_response_back(stream, shl_stm_rx));

    // TODO: Unlock read/write signal

    if let Err(e) = receive_handle.join() {
        println!("Command receiver thread panicked with message {:?}", e);
    };
    if let Err(e) = response_handle.join() {
        println!("Response relayer thread panicked with message {:?}", e);
    };
}

fn receive_and_pass_along_line(stream: TcpStream, stm_shl: Sender<String>) {
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
                stm_shl.send(line).unwrap();
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

fn relay_response_back(mut stream: TcpStream, shl_stm_rx: Receiver<String>) {
    println!(
        "Thread {:?} starting to read response lines",
        thread::current().id()
    );

    while let Ok(output) = shl_stm_rx.recv() {
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

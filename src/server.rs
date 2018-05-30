use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Output};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use command::*;
use serde_json;

pub fn spawn_bash_and_listen() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let (stm_shl_sx, stm_shl_rx) = channel::<Message>();
    let shl_stm_sxs = Arc::new(Mutex::new(Vec::new()));

    let sxs = shl_stm_sxs.clone();
    thread::spawn(move || {
        handle_incoming_streams(sxs, listener, stm_shl_sx);
    });

    loop {
        pipe_stream_to_shell_and_relay_response(&stm_shl_rx, &shl_stm_sxs);
    }
}

fn run_command(content: &str) -> Result<Output, String> {
    let mut words = content.split_whitespace();
    if let Some(cmd) = words.next() {
        let mut process = Command::new(cmd);
        while let Some(arg) = words.next() {
            process.arg(arg);
        }

        match process.output() {
            Ok(output) => {
                {
                    let stdout = ::std::str::from_utf8(&output.stdout).expect("Non-utf8 stdout");
                    let stderr = ::std::str::from_utf8(&output.stderr).expect("Non-utf8 stderr");

                    println!("MAIN: shell returned stdout {:?}, relaying...", stdout);
                    if stderr != "" {
                        println!(
                            "MAIN: shell returned stderr {:?}, thought you should know...",
                            stderr
                        );
                    }
                }

                Ok(output)
            }
            Err(_) => Err(format!("MAIN: bad input: {:?}", content)),
        }
    } else {
        Err("Empty input".to_owned())
    }
}

fn pipe_stream_to_shell_and_relay_response(
    stm_shl_rx: &Receiver<Message>,
    shl_stm_sxs: &Arc<Mutex<Vec<Sender<Message>>>>,
) {
    let input = stm_shl_rx.recv().expect("Nothing to receive");

    let content = match input.mode {
        Mode::Chat => {
            let chat = input.content;
            println!("MAIN: received chat: {:?}", chat);
            chat
        }
        Mode::Cmd => {
            let cmd_string = ::std::str::from_utf8(&input.content).unwrap();

            println!("MAIN: received command: {:?}", cmd_string);

            match run_command(&cmd_string) {
                Ok(resp) => resp.stdout, // TODO: merge stdout and stderr
                Err(e) => format!("Error running command: {}", e).into_bytes(),
            }
        }
    };

    let response = Message {
        content,
        mode: input.mode,
    };

    let mut guard = shl_stm_sxs.lock().expect("Poisoned Vec of outgoing sxs");
    guard.retain(|shl_stm_sx| shl_stm_sx.send(response.clone()).is_ok());

    println!("MAIN: {} clients relayed to", guard.len());
}

fn handle_incoming_streams(
    shl_stm_sxs: Arc<Mutex<Vec<Sender<Message>>>>,
    listener: TcpListener,
    stm_shl_sx: Sender<Message>,
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
    stm_shl_sx: Sender<Message>,
    shl_stm_sxs: Arc<Mutex<Vec<Sender<Message>>>>,
    stream: TcpStream,
) {
    // TODO: lock some item to prevent sending/receiving while threads spin up

    println!(
        "{:?}: Received connection from {}!",
        thread::current().id(),
        stream.peer_addr().expect("Failed to get stream remote IP")
    );

    let alive = Arc::new(Mutex::new(true));

    // Handle reading from the stream
    let read_stream = stream.try_clone().unwrap();
    let al = alive.clone();
    let receive_handle = thread::spawn(move || {
        receive_and_pass_along_line(read_stream, stm_shl_sx, al);
    });

    // Handle writing to the stream
    let (shl_stm_sx, shl_stm_rx) = channel::<Message>();
    {
        shl_stm_sxs.lock().unwrap().push(shl_stm_sx);
    }
    let response_handle = thread::spawn(move || relay_response_back(stream, shl_stm_rx, alive));

    // TODO: Unlock read/write signal

    if let Err(e) = receive_handle.join() {
        println!(
            "{:?}: Command receiver thread panicked with message {:?}",
            thread::current().id(),
            e
        );
    };
    if let Err(e) = response_handle.join() {
        println!(
            "{:?}: Response relayer thread panicked with message {:?}",
            thread::current().id(),
            e
        );
    };
}

fn receive_and_pass_along_line(
    stream: TcpStream,
    stm_shl: Sender<Message>,
    alive: Arc<Mutex<bool>>,
) {
    println!(
        "{:?} reading input from {}",
        thread::current().id(),
        stream.peer_addr().unwrap()
    );

    let mut lines = BufReader::new(&stream).lines();
    while let Some(maybe_line) = lines.next() {
        match maybe_line {
            Ok(mut line) => {
                let user_input = serde_json::from_str::<Message>(&line).unwrap();
                stm_shl.send(user_input).unwrap();
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

    *alive.lock().unwrap() = false;
    println!(
        "Stream in {:?} is closed, marking closed and shutting down...",
        thread::current().id()
    );
}

fn relay_response_back(
    mut stream: TcpStream,
    shl_stm_rx: Receiver<Message>,
    alive: Arc<Mutex<bool>>,
) {
    println!(
        "{:?} relaying responses to {}",
        thread::current().id(),
        stream.peer_addr().unwrap(),
    );

    while let Ok(output) = shl_stm_rx.recv() {
        {
            if !*alive.lock().unwrap() {
                println!(
                    "Stream in thread {:?} is closed, shutting down...",
                    thread::current().id()
                );
                return;
            }
        }

        let mut ser = serde_json::to_vec(&output).unwrap();
        ser.push(b'\n');

        match stream.write(&ser) {
            Err(e) => {
                panic!(
                    "Stream in {:?} failed to write with error {}",
                    thread::current().id(),
                    e
                );
            }
            Ok(_) => {}
        }
    }

    println!(
        "{:?} receiver closed, shutting down...",
        thread::current().id()
    );
}

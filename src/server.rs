use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use serde_json;
use tokio;
use tokio::net::TcpListener;
use tokio::prelude::{future, stream, AsyncRead, Future, Sink, Stream};
use tokio_io::codec::LinesCodec;

use messages::*;

pub fn spawn_bash_and_listen_future() {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("Failed to bind listener");

    let (stm_shl_sx, stm_shl_rx) = channel::<Message>();
    let shl_stm_sxs = Arc::new(Mutex::new(Vec::<Sender<Response>>::new()));

    let shl_stm_sxs_cl = shl_stm_sxs.clone();
    let client_handler = listener
        .incoming()
        .map_err(|e| eprintln!("Incoming failed: {:?}", e))
        .for_each(move |sock| {
            let shl_stm_sxs_cl = shl_stm_sxs_cl.clone();
            let (sink, stream) = sock.framed(LinesCodec::new()).split();

            println!("Received connection..."); // TODO: add more info to log

            //             let user_input = Message {
            //                 content: "dongle".to_owned(),
            //                 mode: Mode::Chat,
            //                 user_name: "sasha".to_owned(),
            //             };
            //             future::ok::<(), ()>({ println!("dongle {:?}", user_input) })

            let stm_shl_sx = stm_shl_sx.clone();
            let input_handler = stream
                .map_err(|e| format!("Failed to read line: {:?}", e))
                .for_each(move |line| {
                    println!("{}", line);

                    // let user_input = serde_json::from_str::<Message>(&line).unwrap();
                    let user_input = Message {
                        content: "dongle".to_owned(),
                        mode: Mode::Chat,
                        user_name: "sasha".to_owned(),
                    };

                    future::result(
                        stm_shl_sx
                            .send(user_input)
                            .map_err(|e| format!("Failed to send: {:?}", e)),
                    )
                });

            let (shl_stm_sx, shl_stm_rx) = channel::<Response>();
            {
                shl_stm_sxs_cl
                    .lock()
                    .expect("Poisoned Vec of shell -> stream Senders")
                    .push(shl_stm_sx);
            }
            let response_handler = future::loop_fn(shl_stm_rx, move |shl_stm_rx| {
                if let Ok(resp) = shl_stm_rx.recv() {
                    let mut ser = serde_json::to_string(&resp).unwrap();
                    ser.push_str("\n");

                    sink.send(ser)
                        .wait()
                        .map_err(|e| println!("Failed to write response: {:?}", e))
                        .map(|_| future::Loop::Continue(shl_stm_rx))
                } else {
                    Ok(future::Loop::Break(()))
                }
            });

            // input_handler
            //     .join(response_handler)
            //     .map_err(|e| eprintln!("Client handler errored: {:?}", e))
            //     .map(|_| println!("Client handler succeeded"))

            future::ok::<(), ()>(println!("asdf"))
        });

    let user_input = Message {
        content: "dongle".to_owned(),
        mode: Mode::Chat,
        user_name: "sasha".to_owned(),
    };

    let shell_handler = future::loop_fn(stm_shl_rx, move |stm_shl_rx| {
        if let Ok(input) = stm_shl_rx.recv() {
            let response_content = match input.mode {
                Mode::Chat => {
                    println!(
                        "MAIN: received chat from {:?}: {:?}",
                        input.user_name, input.content
                    );
                    input.content.clone()
                }
                Mode::Cmd => {
                    println!(
                        "MAIN: received command from {:?}: {:?}",
                        input.user_name, input.content
                    );

                    "yay".to_owned()
                    // match run_command(&input.content) {
                    //     Ok(resp) => resp,
                    //     Err(e) => format!("Error running command: {}", e),
                    // }
                }
            };

            let response = Response {
                og_msg: input,
                response: response_content,
            };

            let mut guard = shl_stm_sxs.lock().expect("Poisoned Vec of outgoing sxs");
            guard.retain(|shl_stm_sx| shl_stm_sx.send(response.clone()).is_ok());

            Ok(future::Loop::Continue(stm_shl_rx))
        } else {
            Ok(future::Loop::Break({ println!("Shell handler is done") }))
        }
    });

    tokio::run(
        client_handler
            .join(shell_handler)
            .map(|_| println!("Shutting down...")),
    );
}

/*
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

fn run_command(content: &str) -> Result<String, String> {
    let mut words = content.split_whitespace();
    if let Some(cmd) = words.next() {
        let mut process = Command::new(cmd);
        while let Some(arg) = words.next() {
            process.arg(arg);
        }

        match process.output() {
            Ok(output) => {
                let stdout = ::std::str::from_utf8(&output.stdout).expect("Non-utf8 stdout");
                let stderr = ::std::str::from_utf8(&output.stderr).expect("Non-utf8 stderr");

                println!("MAIN: shell returned stdout {:?}, relaying...", stdout);
                if stderr != "" {
                    println!(
                        "MAIN: shell returned stderr {:?}, thought you should know...",
                        stderr
                    );
                }

                Ok(stdout.to_owned()) // TODO: merge stdout and stderr
            }
            Err(_) => {
                println!("Bad input: {:?}", content);
                Err(format!("Bad input: {:?}", content))
            }
        }
    } else {
        println!("Empty input");
        Err("Empty input".to_owned())
    }
}

fn pipe_stream_to_shell_and_relay_response(
    stm_shl_rx: &Receiver<Message>,
    shl_stm_sxs: &Arc<Mutex<Vec<Sender<Response>>>>,
) {
    let input = stm_shl_rx.recv().expect("Nothing to receive");

    let response = match input.mode {
        Mode::Chat => {
            println!(
                "MAIN: received chat from {:?}: {:?}",
                input.user_name, input.content
            );
            input.content.clone()
        }
        Mode::Cmd => {
            println!(
                "MAIN: received command from {:?}: {:?}",
                input.user_name, input.content
            );

            match run_command(&input.content) {
                Ok(resp) => resp,
                Err(e) => format!("Error running command: {}", e),
            }
        }
    };

    let response = Response {
        og_msg: input,
        response,
    };

    let mut guard = shl_stm_sxs.lock().expect("Poisoned Vec of outgoing sxs");
    guard.retain(|shl_stm_sx| shl_stm_sx.send(response.clone()).is_ok());

    println!("MAIN: {} clients relayed to", guard.len());
}

fn handle_incoming_streams(
    shl_stm_sxs: Arc<Mutex<Vec<Sender<Response>>>>,
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
    shl_stm_sxs: Arc<Mutex<Vec<Sender<Response>>>>,
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
    let (shl_stm_sx, shl_stm_rx) = channel::<Response>();
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
    shl_stm_rx: Receiver<Response>,
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
*/

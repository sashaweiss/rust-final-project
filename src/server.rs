use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use super::{DeserializeOwned, Serialize};
use serde_json;

/// Trait implemented by a struct to define customizable functionality for a synchronous terminal server.
///
pub trait ShellServer<M, R>
where
    M: DeserializeOwned + Send + 'static,
    R: Serialize + Send + 'static + Clone,
{
    /// Process input from user before relaying it to other clients.
    ///
    /// # Examples
    /// ```no_run
    /// fn process_input(&self, input: Message) -> Response {
    ///        let response = match input.mode {
    ///            Mode::Upper => {
    ///                let mut s = input.content.to_uppercase().to_owned();
    ///                s.push_str("!!!");
    ///                s
    ///            }
    ///            Mode::Lower => {
    ///                input.content.to_lowercase().to_owned()
    ///            }
    ///        };
    ///
    ///        Response {
    ///            og_msg: input,
    ///            response,
    ///        }
    ///    }
    ///
    /// ```
    fn process_input(&self, M) -> R;
}

/// Takes in an instances of a ShellServer, and starts a server that synchronous terminals clients
/// can connect to.
///
/// # Examples
/// ```no_run
/// syncterm::server::spawn_shell_and_listen(server::App());
/// ```
///
pub fn spawn_shell_and_listen<M, R, S>(server: S)
where
    M: DeserializeOwned + Send + 'static,
    R: Serialize + Send + 'static + Clone,
    S: ShellServer<M, R>,
{
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    let (stm_shl_sx, stm_shl_rx) = channel::<M>();
    let shl_stm_sxs = Arc::new(Mutex::new(Vec::new()));

    let sxs = shl_stm_sxs.clone();
    thread::spawn(move || {
        handle_incoming_streams(sxs, listener, stm_shl_sx);
    });

    loop {
        pipe_stream_to_shell_and_relay_response(&stm_shl_rx, &shl_stm_sxs, &server);
    }
}

fn pipe_stream_to_shell_and_relay_response<M, R, S>(
    stm_shl_rx: &Receiver<M>,
    shl_stm_sxs: &Arc<Mutex<Vec<Sender<R>>>>,
    server: &S,
) where
    M: DeserializeOwned + Send + 'static,
    R: Serialize + Send + 'static + Clone,
    S: ShellServer<M, R>,
{
    let input = stm_shl_rx.recv().expect("Nothing to receive");

    let response = server.process_input(input);

    let mut guard = shl_stm_sxs.lock().expect("Poisoned Vec of outgoing sxs");
    guard.retain(|shl_stm_sx| shl_stm_sx.send(response.clone()).is_ok());

    println!("MAIN: {} clients relayed to", guard.len());
}

fn handle_incoming_streams<M, R>(
    shl_stm_sxs: Arc<Mutex<Vec<Sender<R>>>>,
    listener: TcpListener,
    stm_shl_sx: Sender<M>,
) where
    M: DeserializeOwned + Send + 'static,
    R: Serialize + Send + 'static + Clone,
{
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

fn handle_new_stream<M, R>(
    stm_shl_sx: Sender<M>,
    shl_stm_sxs: Arc<Mutex<Vec<Sender<R>>>>,
    stream: TcpStream,
) where
    M: DeserializeOwned + Send + 'static,
    R: Serialize + Send + 'static + Clone,
{
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
    let (shl_stm_sx, shl_stm_rx) = channel::<R>();
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

fn receive_and_pass_along_line<M>(stream: TcpStream, stm_shl: Sender<M>, alive: Arc<Mutex<bool>>)
where
    M: DeserializeOwned + Send + 'static,
{
    println!(
        "{:?} reading input from {}",
        thread::current().id(),
        stream.peer_addr().unwrap()
    );

    let mut lines = BufReader::new(&stream).lines();
    while let Some(maybe_line) = lines.next() {
        match maybe_line {
            Ok(mut line) => {
                let user_input = serde_json::from_str::<M>(&line).unwrap();
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

fn relay_response_back<R>(mut stream: TcpStream, shl_stm_rx: Receiver<R>, alive: Arc<Mutex<bool>>)
where
    R: Serialize + Send + 'static + Clone,
{
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

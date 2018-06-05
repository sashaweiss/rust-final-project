use std::io;
use std::thread;

use chan;
use termion::input::TermRead;

use super::{DeserializeOwned, Serialize};
use shell_connection::ShellConnection;

pub enum KeyAction<M: Serialize> {
    DoNothing,
    Exit,
    SendMessage(M),
}

pub trait ShellClient<M, R>
where
    M: Serialize,
    R: DeserializeOwned + Send,
{
    fn on_key(&mut self, super::Key) -> KeyAction<M>;
    fn receive_response(&mut self, R);
    fn draw(&mut self);
    fn first_draw(&mut self);
    fn last_draw(&mut self);
}

pub fn connect<C, M, R>(client: C)
where
    M: Serialize,
    R: DeserializeOwned + Send + 'static,
    C: ShellClient<M, R>,
{
    let mut connection = ShellConnection::connect("127.0.0.1:8080").unwrap();

    render(&mut connection, client);
}

fn render<C, M, R>(connection: &mut ShellConnection, mut client: C)
where
    M: Serialize,
    R: DeserializeOwned + Send + 'static,
    C: ShellClient<M, R>,
{
    // Input thread
    let (input_tx, input_rx) = chan::sync(0);
    thread::spawn(move || {
        let stdin = io::stdin();
        for c in stdin.keys() {
            let evt = c.unwrap();
            input_tx.send(evt);
        }
    });

    // Connection reading thread
    let (response_tx, response_rx) = chan::sync(0);
    let read_connection = connection.try_clone().unwrap();
    thread::spawn(move || loop {
        match read_connection.read_response() {
            Ok(resp) => response_tx.send(resp),
            Err(_) => break,
        };
    });

    client.first_draw();

    loop {
        chan_select! {
            input_rx.recv() -> key => {
                match client.on_key(key.unwrap()) {
                    KeyAction::DoNothing => {}
                    KeyAction::Exit => break,
                    KeyAction::SendMessage(msg) => {
                        connection.send_input(msg).unwrap();
                    }
                }
            },
            response_rx.recv() -> response => {
                if let Some(response) = response {
                    client.receive_response(response);
                } else {
                    break;
                }
            },
        }

        client.draw();
    }

    client.last_draw();
}

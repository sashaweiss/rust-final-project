/// This file borrowed from the `tui-rs` examples, and modified for our purposes.
/// See: https://github.com/fdehau/tui-rs/blob/master/examples/user_input.rs
use std::io;
use std::thread;

use chan;
use rand::random;
use termion::input::TermRead;

use messages::*;
use shell_connection::ShellConnection;

pub enum KeyAction {
    DoNothing,
    Exit,
    SendMessage(Message), // TODO: Message -> Serializable
}

pub trait ShellClient: Sized {
    fn on_key(&mut self, super::Key) -> KeyAction;
    fn receive_response(&mut self, Response); // TODO: Response -> Deserializable
    fn draw(&mut self);
    fn first_draw(&mut self);
    fn last_draw(&mut self);
}

pub fn connect<C: ShellClient>(client: C) {
    let mut connection = ShellConnection::connect("127.0.0.1:8080").unwrap();

    // render(&mut connection, );
}

fn render<C: ShellClient>(connection: &mut ShellConnection, mut client: C) {
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

use std::io;
use std::thread;

use chan;
use termion::input::TermRead;

use super::{DeserializeOwned, Serialize};
use shell_connection::ShellConnection;

/// Returned by ShellClient::on_key to specify an API action to be triggered after a key is pressed.
pub enum KeyAction<M: Serialize> {
    DoNothing,
    /// Exits from synced terminal
    Exit,
    /// Sends a user's input to the server defined by ShellServer
    SendMessage(M),
}


/// Trait implemented by a struct to define customizable functionality for a synchronous terminal client.
///
/// # Examples
/// ```
///
/// ```
///
pub trait ShellClient<M, R>
    where
        M: Serialize,
        R: DeserializeOwned + Send,
{    /// Given a key press, defines actions to take. Returns a KeyAction to trigger additional API actions.
    ///
    /// # Examples
    /// ```
    ///
    /// ```
    ///
    fn on_key(&mut self, super::Key) -> KeyAction<M>;

    /// When client receives a response from the server, defines any actions to take.
    ///
    /// # Examples
    /// ```
    ///
    /// ```
    ///
    fn receive_response(&mut self, R);

    /// Initializes the client UI.
    ///
    /// # Examples
    /// ```
    ///
    /// ```
    ///
    fn first_draw(&mut self);

    /// Updates the client UI (called in a loop).
    ///
    /// # Examples
    /// ```
    ///
    /// ```
    ///
    fn draw(&mut self);

    /// Client UI right after exiting shared terminal.
    ///
    /// # Examples
    /// ```
    ///
    /// ```
    ///
    fn last_draw(&mut self);
}

/// Takes in an instances of a Shell Client and connects to the server.
/// /// # Examples
/// ```
///
/// ```
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

use std::io;
use std::thread;

use chan;
pub use termion::event::Key;
use termion::input::TermRead;

use serde::{Serialize, de::DeserializeOwned};
use shell_connection::ShellConnection;

/// Returned by `ShellClient::on_key` to specify an API action to be triggered after a key is pressed.
///
/// M must implement `serde::Serialize` to allow robust client-server communication.
/// We recommend using the `serde_derive` crate and its `#[derive(Serialize)]` macro to achieve this.
#[derive(Debug, Clone)]
pub enum KeyAction<M: Serialize> {
    DoNothing,
    /// Exits from synced terminal
    Exit,
    /// Sends a user's input to the server defined by ShellServer
    SendMessage(M),
}

/// Trait implemented by a struct to define customizable functionality for a synchronous terminal client.
///
/// M and R must implement `serde::Serialize` and `serde::de::DeserializeOwned`, respectively, to allow
/// robust client-server communication. We recommend using the `serde_derive` crate and its
/// `#[derive(Serialize, Deserialize)]` macros to achieve this.
pub trait ShellClient<M, R>
where
    M: Serialize,
    R: DeserializeOwned + Send,
{
    /// Returns the URL of the shell server to connect to.
    fn server_url(&self) -> String;

    /// Given a key press, defines actions to take.
    /// Returns a [KeyAction](enum.KeyAction.html) to signal next library action.
    ///
    /// # Examples
    /// ```
    /// # use syncterm::Key;
    /// # use syncterm::client::KeyAction;
    ///
    /// fn on_key(&mut self, key: Key) -> KeyAction<Message> {
    ///        match key {
    ///            Key::Ctrl('c') | Key::Esc => {
    ///                return KeyAction::Exit;
    ///            }
    ///            Key::Char('\n') => {
    ///                let message = self.input_buffer.drain(..).collect::<String>();
    ///                return KeyAction::SendMessage(message);
    ///            }
    ///            Key::Char(c) => {
    ///                self.input_buffer.push(c);
    ///            }
    ///            Key::Backspace => {
    ///                self.input_buffer.pop();
    ///            }
    ///            _ => {}
    ///        }
    ///
    ///        KeyAction::DoNothing
    ///    }
    /// ```
    fn on_key(&mut self, key: Key) -> KeyAction<M>;

    /// When client receives a response from the server, defines any actions to take.
    ///
    /// # Examples
    /// ```no_run
    /// fn receive_response(&mut self, response: String) {
    ///     self.messages.push(response);
    ///}
    /// ```
    ///
    fn receive_response(&mut self, server_response: R);

    /// Does any work to initialize the client UI.
    fn first_draw(&mut self);

    /// Updates the client UI (called in an animation-style update loop).
    ///
    /// # Examples
    /// ```
    /// fn draw(&mut self) {
    ///     if let Some(m) = self.messages.pop() {
    ///         println!("Message: {}", m);
    ///     }
    /// }
    /// ```
    fn draw(&mut self);

    /// Does any work to tear-down the client UI.
    fn last_draw(&mut self);
}

/// The "main" function for ShellClients.
///
/// Captures stdin, uses the ShellClient to send messages to and receive responses from
/// a server, and runs an animation update loop to render the UI.
///
/// Returns an error only if the client's `server_url` fails to connect.
pub fn connect<C, M, R>(client: C) -> Result<(), String>
where
    M: Serialize,
    R: DeserializeOwned + Send + 'static,
    C: ShellClient<M, R>,
{
    let mut connection = ShellConnection::connect("127.0.0.1:8080")
        .map_err(|e| format!("Failed to connect to server: {:?}", e))?;

    render(&mut connection, client);
    Ok(())
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

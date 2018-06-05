extern crate chrono;
#[macro_use]
extern crate serde_derive;
extern crate syncterm;
extern crate termion;
extern crate tui;

mod client;
mod messages;
mod server;

fn main() {
    let mut args = ::std::env::args();
    args.next();

    if let Some(name) = args.next() {
        syncterm::client::connect(client::App::new(name));
    } else {
        syncterm::server::spawn_shell_and_listen(server::App());
    }
}

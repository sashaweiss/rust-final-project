/// This file borrowed from the `tui-rs` examples, and modified for our purposes.
/// See: https://github.com/fdehau/tui-rs/blob/master/examples/user_input.rs
///
/// A simple example demonstrating how to handle user input. This is
/// a bit out of the scope of the library as it does not provide any
/// input handling out of the box. However, it may helps some to get
/// started.
///
/// This is a very simple example:
///   * A input box always focused. Every character you type is registered
///   here
///   * Pressing Backspace erases a character
///   * Pressing Enter pushes the current input in the history of previous
///   messages
extern crate termion;
extern crate tui;

use std::io;
use std::net::TcpStream;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use termion::event;
use termion::input::TermRead;

use tui::Terminal;
use tui::backend::MouseBackend;
use tui::layout::{Direction, Group, Rect, Size};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Item, List, Paragraph, Widget};

use client::ShellConnection;
use command::*;

struct App {
    size: Rect,
    input: String,
    messages: Arc<Mutex<Vec<(String, String)>>>,
}

impl App {
    fn new() -> App {
        App {
            size: Rect::default(),
            input: String::new(),
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

enum Event {
    Input(event::Key),
}

pub fn render(connection: &mut ShellConnection<TcpStream>, user_name: &str) {
    // Terminal initialization
    let backend = MouseBackend::new().unwrap();
    let mut terminal = Terminal::new(backend).unwrap();

    // Channels
    let (tx, rx) = mpsc::channel();
    let input_tx = tx.clone();

    // Input
    thread::spawn(move || {
        let stdin = io::stdin();
        for c in stdin.keys() {
            let evt = c.unwrap();
            input_tx.send(Event::Input(evt)).unwrap();
            if evt == event::Key::Char('q') {
                break;
            }
        }
    });

    // App
    let mut app = App::new();

    let messages_thread = app.messages.clone();
    let read_connection = connection.try_clone().unwrap();
    thread::spawn(move || loop {
        let mut resp = read_connection.read_response();

        messages_thread
            .lock()
            .expect("Poisoned messages")
            .push((resp.user_name, resp.content));
    });

    // First draw call
    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();
    app.size = terminal.size().unwrap();
    draw(&mut terminal, &app);

    let mut mode = Mode::Chat;

    loop {
        let size = terminal.size().unwrap();
        if app.size != size {
            terminal.resize(size).unwrap();
            app.size = size;
        }

        let evt = rx.recv().unwrap();
        match evt {
            Event::Input(input) => match input {
                event::Key::Char('q') => {
                    break;
                }
                event::Key::Char('\n') => {
                    let message = app.input.drain(..).collect::<String>();
                    match message.as_ref() {
                        "CHAT" => {
                            mode = Mode::Chat;
                            println!("Switched to Chat mode");
                        }
                        "CMD" => {
                            mode = Mode::Cmd;
                            println!("Switched to Cmd mode");
                        }
                        _ => {
                            connection.send_input(&message, &mode, &user_name).unwrap();
                        }
                    }
                }
                event::Key::Char(c) => {
                    app.input.push(c);
                }
                event::Key::Backspace => {
                    app.input.pop();
                }
                _ => {}
            },
        }
        draw(&mut terminal, &app);
    }

    terminal.show_cursor().unwrap();
    terminal.clear().unwrap();
}

fn draw(t: &mut Terminal<MouseBackend>, app: &App) {
    Group::default()
        .direction(Direction::Vertical)
        .margin(2)
        .sizes(&[Size::Fixed(3), Size::Min(1)])
        .render(t, &app.size, |t, chunks| {
            Paragraph::default()
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Input"))
                .text(&app.input)
                .render(t, &chunks[0]);
            List::new(
                app.messages
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(u, m)| Item::Data(format!("{}: {}", u, m))),
            ).block(Block::default().borders(Borders::ALL).title("Messages"))
                .render(t, &chunks[1]);
        });

    t.draw().unwrap();
}

/// This file borrowed from the `tui-rs` examples, and modified for our purposes.
/// See: https://github.com/fdehau/tui-rs/blob/master/examples/user_input.rs
use std::io;
use std::net::TcpStream;
use std::thread;

use termion::event;
use termion::input::TermRead;

use tui::Terminal;
use tui::backend::MouseBackend;
use tui::layout::{Direction, Group, Rect, Size};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Paragraph, Widget};

use chan;

use client::ShellConnection;
use command::*;

struct App {
    size: Rect,
    input: String,
    input_mode: Mode,
    messages: Vec<(String, String)>,
    commands: Vec<(String, String, String)>,
}

impl App {
    fn new() -> App {
        App {
            size: Rect::default(),
            input: String::new(),
            input_mode: Mode::Chat,
            messages: Vec::new(),
            commands: Vec::new(),
        }
    }
}

pub fn render(connection: &mut ShellConnection<TcpStream>, user_name: &str) {
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

    // Terminal drawing thread
    let backend = MouseBackend::new().unwrap();
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new();

    // First draw call
    terminal.clear().unwrap();
    terminal.hide_cursor().unwrap();
    app.size = terminal.size().unwrap();
    draw(&mut terminal, &app);

    loop {
        // Adjust size if necessary
        let size = terminal.size().unwrap();
        if app.size != size {
            terminal.resize(size).unwrap();
            app.size = size;
        }

        chan_select! {
            input_rx.recv() -> key => {
                match key.unwrap() {
                    event::Key::Ctrl('c') | event::Key::Esc => {
                        break;
                    }
                    event::Key::Char('\n') => {
                        let message = app.input.drain(..).collect::<String>();
                        match message.as_ref() {
                            "CHAT" => {
                                app.input_mode = Mode::Chat;
                            }
                            "CMD" => {
                                app.input_mode = Mode::Cmd;
                            }
                            _ => {
                                connection.send_input(&message, &app.input_mode, &user_name).unwrap();
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
                }
            },
            response_rx.recv() -> response => {
                if let Some(response) = response {
                    match response.og_msg.mode {
                        Mode::Chat => {
                            app.messages.push((response.og_msg.user_name, response.response));
                        }
                        Mode::Cmd => {
                            app.commands.push((response.og_msg.user_name, response.og_msg.content, response.response));
                        }
                    };
                } else {
                    break;
                }
            },
        }

        draw(&mut terminal, &app);
    }

    terminal.show_cursor().unwrap();
}

fn draw(t: &mut Terminal<MouseBackend>, app: &App) {
    Group::default()
        .direction(Direction::Vertical)
        .margin(2)
        .sizes(&[Size::Fixed(3), Size::Min(1)])
        .render(t, &app.size, |t, chunks| {
            Paragraph::default()
                .style(Style::default().fg(Color::Yellow))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(match app.input_mode {
                            Mode::Chat => "Chat",
                            Mode::Cmd => "Command",
                        }),
                )
                .text(&app.input)
                .render(t, &chunks[0]);

            Group::default()
                .direction(Direction::Horizontal)
                .margin(0)
                .sizes(&[Size::Percent(50), Size::Percent(50)])
                .render(t, &chunks[1], |t, chunks| {
                    // Use Paragraphs so we can get text wrapping
                    let messages: String = app.messages.iter().rev().fold(
                        "".to_owned(),
                        |mut acc, (u, m)| {
                            acc.push_str(&format!("{}: {}\n", u, m));
                            acc
                        },
                    );
                    Paragraph::default()
                        .block(Block::default().borders(Borders::ALL).title("Messages"))
                        .wrap(true)
                        .text(&messages)
                        .render(t, &chunks[0]);

                    let commands: String = app.commands.iter().rev().enumerate().fold(
                        "".to_owned(),
                        |mut acc, (i, (u, c, m))| {
                            acc.push_str(&format!(
                                "{}: {} >> {}\n{}{}\n",
                                i,
                                u,
                                c,
                                m,
                                if m.ends_with("\n") { "" } else { "\n" }
                            ));
                            acc
                        },
                    );
                    Paragraph::default()
                        .block(Block::default().borders(Borders::ALL).title("Commands"))
                        .wrap(true)
                        .text(&commands)
                        .render(t, &chunks[1]);
                });
        });

    t.draw().unwrap();
}

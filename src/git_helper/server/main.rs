extern crate syncterm;

use syncterm::messages::*;
use syncterm::server::*;

use std::process::Command;

struct App {}

impl App {
    fn new() -> App {
        App {}
    }
}

impl ShellServer for App {
    fn process_input(&self, input: Message) -> Result<Response, String> {
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
                    Err(e) => return Err(format!("Error running command: {}", e)),
                }
            }
        };

        Ok(Response {
            og_msg: input,
            response,
        })
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

fn main() {
    spawn_bash_and_listen(App::new());
}

use std::process::Command;

use messages::*;

use syncterm;

pub struct App();

impl syncterm::server::ShellServer<Message, Response> for App {
    fn local_address(&self) -> String {
        "127.0.0.1:8080".to_owned()
    }

    fn process_input(&self, input: Message) -> Response {
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

        Response {
            og_msg: input,
            response,
        }
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

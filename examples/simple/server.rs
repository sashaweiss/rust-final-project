use messages::*;

use syncterm;

pub struct App();

impl syncterm::server::ShellServer<Message, Response> for App {
    fn local_address(&self) -> String {
        "127.0.0.1:8080".to_owned()
    }

    fn process_input(&self, input: Message) -> Response {
        println!(
            "MAIN: received chat from {:?}: {:?}",
            input.user_name, input.content
        );
        let response = match input.mode {
            Mode::Upper => {
                let mut s = input.content.to_uppercase().to_owned();
                s.push_str("!!!");
                s
            }
            Mode::Lower => input.content.to_lowercase().to_owned(),
        };

        Response {
            og_msg: input,
            response,
        }
    }
}

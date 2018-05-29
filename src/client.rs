use std::io::{stdin, BufRead, BufReader, Read, Result, Write};
use std::net::TcpStream;
use std::sync::mpsc::channel;
use std::thread;

pub struct CommandResponse {
    response: String,
    // exit_status: usize,
}

pub struct CommandResponseLines {
    lines: Vec<String>,
}

impl<'a> Iterator for CommandResponseLines {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.lines.pop()
    }
}

impl IntoIterator for CommandResponse {
    type Item = String;
    type IntoIter = CommandResponseLines;

    fn into_iter(self) -> Self::IntoIter {
        CommandResponseLines {
            lines: self.response.split('\n').map(|l| l.to_owned()).collect(),
        }
    }
}

pub struct ShellConnection<S: Read + Write> {
    stream: S,
    remote_url: String,
}

impl ShellConnection<TcpStream> {
    pub fn connect(url: &str) -> Result<Self> {
        let stream = TcpStream::connect(url)?;

        Ok(Self {
            stream,
            remote_url: url.to_owned(),
        })
    }

    pub fn try_clone(&self) -> Result<Self> {
        let stream_clone = self.stream.try_clone()?;

        Ok(Self {
            stream: stream_clone,
            remote_url: self.remote_url.clone(),
        })
    }

    pub fn send_command(&mut self, cmd: &str) -> Result<usize> {
        let n_bytes = self.stream.write(cmd.as_bytes())?;
        Ok(n_bytes + self.stream.write(&[b'\n'])?)
    }

    pub fn read_response(&self) -> CommandResponse {
        let mut resp_lines = Vec::new();
        let mut lines = BufReader::new(&self.stream).lines();
        while let Some(Ok(line)) = lines.next() {
            if line == "END OF MESSAGE" {
                break;
            }

            resp_lines.push(line);
        }

        let response = resp_lines.join("\n").trim().to_owned();
        CommandResponse {
            response,
            // exit_status: 0,
        }
    }
}

pub fn connect_and_echo() {
    let mut connection = ShellConnection::connect("127.0.0.1:8080").unwrap();
    let mut read_connection = connection.try_clone().unwrap();

    thread::spawn(move || {
        let mut lines = BufReader::new(stdin()).lines();

        while let Some(Ok(line)) = lines.next() {
            connection.send_command(&line).unwrap();
        }
    });

    loop {
        let resp = read_connection.read_response();
        println!("Response: {}", resp.response);

        /*
         * for line in resp {
         *   println!("Line: {}", line);
         * }
         */
    }
}

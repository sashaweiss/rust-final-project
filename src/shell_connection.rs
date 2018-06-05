use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;

use serde_json;

use serde::{Serialize, de::DeserializeOwned};

pub(crate) struct ShellConnection {
    stream: TcpStream,
    remote_url: String,
}

impl ShellConnection {
    pub fn connect(url: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(url)?;

        Ok(Self {
            stream,
            remote_url: url.to_owned(),
        })
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        let stream_clone = self.stream.try_clone()?;

        Ok(Self {
            stream: stream_clone,
            remote_url: self.remote_url.clone(),
        })
    }

    pub fn send_input<M: Serialize>(&mut self, msg: M) -> io::Result<usize> {
        let mut sendable = serde_json::to_vec(&msg).unwrap();
        sendable.push(b'\n');

        self.stream.write(&sendable)
    }

    pub fn read_response<R: DeserializeOwned>(&self) -> Result<R, String> {
        let mut resp = String::new();
        BufReader::new(&self.stream)
            .read_line(&mut resp)
            .map_err(|e| format!("Error reading: {:?}", e))?;

        serde_json::from_str(&resp).map_err(|e| format!("Error reading: {:?}", e))
    }
}

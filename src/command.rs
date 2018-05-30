#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandResponse {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInput {
    pub content: Vec<u8>,
    pub mode: Mode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Mode {
    Chat,
    Cmd,
}

impl Mode {
    pub fn prompt(&self) -> String {
        match self {
            &Mode::Chat => "CHAT >>".to_string(),
            &Mode::Cmd => ">>".to_string(),
        }
    }
}

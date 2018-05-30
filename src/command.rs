#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
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
            &Mode::Chat => "CHAT >> ".to_string(),
            &Mode::Cmd => ">> ".to_string(),
        }
    }
}

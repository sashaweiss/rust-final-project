#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub content: String,
    pub mode: Mode,
    pub user_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub og_msg: Message,
    pub response: String,
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

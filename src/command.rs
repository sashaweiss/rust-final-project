#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandResponse {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

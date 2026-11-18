use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LocalProviderModels {
    pub key: String,
    pub running: bool,
    pub models: Vec<String>,
}

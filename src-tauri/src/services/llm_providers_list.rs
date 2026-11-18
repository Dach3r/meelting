use crate::catalogs::llm::{self, Provider};

pub fn run() -> Vec<Provider> {
    llm::providers()
}

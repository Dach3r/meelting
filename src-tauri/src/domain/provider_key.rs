pub fn secret_name(workspace_id: &str, provider: &str) -> String {
    format!("{workspace_id}/{provider}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_secret_by_workspace_and_provider() {
        assert_eq!(secret_name("ws-1", "groq"), "ws-1/groq");
    }
}

use keyring::Entry;

const SERVICE: &str = "meelting";

#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    #[error("keychain unavailable")]
    Unavailable(#[from] keyring::Error),
}

fn entry(name: &str) -> Result<Entry, KeychainError> {
    Ok(Entry::new(SERVICE, name)?)
}

pub fn save(name: &str, secret: &str) -> Result<(), KeychainError> {
    Ok(entry(name)?.set_password(secret)?)
}

pub fn read(name: &str) -> Result<Option<String>, KeychainError> {
    match entry(name)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn last_four(name: &str) -> Result<Option<String>, KeychainError> {
    Ok(read(name)?.map(|secret| {
        let chars: Vec<char> = secret.chars().collect();
        let start = chars.len().saturating_sub(4);
        chars[start..].iter().collect()
    }))
}

pub fn remove(name: &str) -> Result<(), KeychainError> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let name = "test/spike-round-trip";
        save(name, "sk-test-abcd1234").unwrap();
        assert_eq!(read(name).unwrap().as_deref(), Some("sk-test-abcd1234"));
        assert_eq!(last_four(name).unwrap().as_deref(), Some("1234"));
        remove(name).unwrap();
        assert_eq!(last_four(name).unwrap(), None);
    }
}

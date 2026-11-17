use keyring::Entry;
use serde::Serialize;

const SERVICE: &str = "meelting";

#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    #[error("keychain unavailable")]
    Unavailable(#[from] keyring::Error),
}

impl Serialize for KeychainError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

fn entry(name: &str) -> Result<Entry, KeychainError> {
    Ok(Entry::new(SERVICE, name)?)
}

pub fn save(name: &str, secret: &str) -> Result<(), KeychainError> {
    Ok(entry(name)?.set_password(secret)?)
}

pub fn last_four(name: &str) -> Result<Option<String>, KeychainError> {
    match entry(name)?.get_password() {
        Ok(secret) => {
            let chars: Vec<char> = secret.chars().collect();
            let start = chars.len().saturating_sub(4);
            Ok(Some(chars[start..].iter().collect()))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.into()),
    }
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
        assert_eq!(last_four(name).unwrap().as_deref(), Some("1234"));
        remove(name).unwrap();
        assert_eq!(last_four(name).unwrap(), None);
    }
}

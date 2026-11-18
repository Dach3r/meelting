use crate::domain::agent_run::{Partial, TokenChunk};
use crate::domain::prompt::Prompt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio_util::sync::CancellationToken;

struct Entry {
    room_id: String,
    attempt: i64,
    seq: u64,
    text: String,
    pending: String,
    prompt: Option<Arc<Prompt>>,
    cancel: CancellationToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLong;

#[derive(Default)]
pub struct Registry {
    entries: Mutex<HashMap<String, Entry>>,
    last_responders: Mutex<HashMap<String, String>>,
}

impl Registry {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Entry>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn responded(&self, room_id: &str, agent_id: &str) {
        self.last_responders
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(room_id.to_string(), agent_id.to_string());
    }

    pub fn last_responder(&self, room_id: &str) -> Option<String> {
        self.last_responders
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(room_id)
            .cloned()
    }

    pub fn insert(&self, run_id: &str, room_id: &str) -> CancellationToken {
        let cancel = CancellationToken::new();
        self.lock().insert(
            run_id.to_string(),
            Entry {
                room_id: room_id.to_string(),
                attempt: 0,
                seq: 0,
                text: String::new(),
                pending: String::new(),
                prompt: None,
                cancel: cancel.clone(),
            },
        );
        cancel
    }

    pub fn start_attempt(&self, run_id: &str, attempt: i64) {
        if let Some(entry) = self.lock().get_mut(run_id) {
            entry.attempt = attempt;
            entry.seq = 0;
            entry.text.clear();
            entry.pending.clear();
        }
    }

    pub fn push(&self, run_id: &str, text: &str, max_bytes: usize) -> Result<(), TooLong> {
        let mut entries = self.lock();
        let Some(entry) = entries.get_mut(run_id) else {
            return Ok(());
        };
        if entry.text.len() + entry.pending.len() + text.len() > max_bytes {
            return Err(TooLong);
        }
        entry.pending.push_str(text);
        Ok(())
    }

    pub fn flush(&self, run_id: &str) -> Option<TokenChunk> {
        let mut entries = self.lock();
        let entry = entries.get_mut(run_id)?;
        if entry.pending.is_empty() || entry.cancel.is_cancelled() {
            return None;
        }
        let text = std::mem::take(&mut entry.pending);
        entry.text.push_str(&text);
        entry.seq += 1;
        Some(TokenChunk {
            run_id: run_id.to_string(),
            room_id: entry.room_id.clone(),
            attempt: entry.attempt,
            seq: entry.seq,
            text,
        })
    }

    pub fn full_text(&self, run_id: &str) -> Option<String> {
        self.lock()
            .get(run_id)
            .map(|entry| format!("{}{}", entry.text, entry.pending))
    }

    pub fn prompt(&self, run_id: &str) -> Option<Arc<Prompt>> {
        self.lock()
            .get(run_id)
            .and_then(|entry| entry.prompt.clone())
    }

    pub fn set_prompt(&self, run_id: &str, prompt: Prompt) -> Option<Arc<Prompt>> {
        let mut entries = self.lock();
        let entry = entries.get_mut(run_id)?;
        let prompt = Arc::new(prompt);
        entry.prompt = Some(prompt.clone());
        Some(prompt)
    }

    pub fn partial(&self, run_id: &str) -> Option<Partial> {
        self.lock()
            .get(run_id)
            .map(|entry| to_partial(run_id, entry))
    }

    pub fn partials(&self, room_id: &str) -> Vec<Partial> {
        let mut partials: Vec<Partial> = self
            .lock()
            .iter()
            .filter(|(_, entry)| entry.room_id == room_id)
            .map(|(run_id, entry)| to_partial(run_id, entry))
            .collect();
        partials.sort_by(|a, b| a.run_id.cmp(&b.run_id));
        partials
    }

    pub fn cancel(&self, run_id: &str) {
        if let Some(entry) = self.lock().remove(run_id) {
            entry.cancel.cancel();
        }
    }

    pub fn cancel_room(&self, room_id: &str) {
        self.lock().retain(|_, entry| {
            let keep = entry.room_id != room_id;
            if !keep {
                entry.cancel.cancel();
            }
            keep
        });
    }

    pub fn remove(&self, run_id: &str) {
        self.lock().remove(run_id);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.lock().len()
    }
}

fn to_partial(run_id: &str, entry: &Entry) -> Partial {
    Partial {
        run_id: run_id.to_string(),
        attempt: entry.attempt,
        seq: entry.seq,
        text: entry.text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_pushed_text_into_numbered_chunks_per_attempt() {
        let registry = Registry::default();
        registry.insert("run1", "r1");
        registry.start_attempt("run1", 1);

        registry.push("run1", "Hel", 100).unwrap();
        registry.push("run1", "lo", 100).unwrap();
        let first = registry.flush("run1").unwrap();
        assert_eq!(registry.flush("run1"), None);
        registry.push("run1", "!", 100).unwrap();
        let second = registry.flush("run1").unwrap();

        assert_eq!(
            (first.attempt, first.seq, first.text.as_str()),
            (1, 1, "Hello")
        );
        assert_eq!(first.room_id, "r1");
        assert_eq!((second.seq, second.text.as_str()), (2, "!"));
        assert_eq!(registry.full_text("run1").as_deref(), Some("Hello!"));
    }

    #[test]
    fn the_snapshot_holds_only_text_already_sent_as_chunks() {
        let registry = Registry::default();
        registry.insert("run1", "r1");
        registry.start_attempt("run1", 1);
        registry.push("run1", "sent", 100).unwrap();
        registry.flush("run1");
        registry.push("run1", " pending", 100).unwrap();

        assert_eq!(
            registry.partial("run1"),
            Some(Partial {
                run_id: "run1".into(),
                attempt: 1,
                seq: 1,
                text: "sent".into()
            })
        );
        assert_eq!(registry.full_text("run1").as_deref(), Some("sent pending"));
    }

    #[test]
    fn a_new_attempt_empties_the_text_and_restarts_the_sequence() {
        let registry = Registry::default();
        registry.insert("run1", "r1");
        registry.start_attempt("run1", 1);
        registry.push("run1", "first try", 100).unwrap();
        registry.flush("run1");

        registry.start_attempt("run1", 2);
        registry.push("run1", "x", 100).unwrap();
        let chunk = registry.flush("run1").unwrap();

        assert_eq!((chunk.attempt, chunk.seq), (2, 1));
        assert_eq!(registry.full_text("run1").as_deref(), Some("x"));
    }

    #[test]
    fn never_holds_more_than_the_byte_limit() {
        let registry = Registry::default();
        registry.insert("run1", "r1");
        registry.push("run1", "ñññ", 8).unwrap();
        registry.flush("run1");

        assert_eq!(registry.push("run1", "ññ", 8), Err(TooLong));
        registry.push("run1", "ñ", 8).unwrap();
        assert_eq!(registry.full_text("run1").unwrap().len(), 8);
    }

    #[test]
    fn snapshots_are_isolated_by_room() {
        let registry = Registry::default();
        registry.insert("run1", "r1");
        registry.insert("run2", "r2");
        registry.insert("run3", "r1");

        let runs: Vec<String> = registry
            .partials("r1")
            .into_iter()
            .map(|partial| partial.run_id)
            .collect();

        assert_eq!(runs, ["run1", "run3"]);
        assert!(registry.partials("r9").is_empty());
    }

    #[test]
    fn cancelling_fires_the_token_and_drops_the_buffer() {
        let registry = Registry::default();
        let first = registry.insert("run1", "r1");
        let second = registry.insert("run2", "r2");
        let third = registry.insert("run3", "r2");

        registry.cancel("run1");
        registry.cancel_room("r2");
        registry.cancel("missing");

        assert!(first.is_cancelled() && second.is_cancelled() && third.is_cancelled());
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.flush("run1"), None);
    }

    #[test]
    fn keeps_the_frozen_prompt_until_removed() {
        let registry = Registry::default();
        registry.insert("run1", "r1");
        let prompt = Prompt {
            system: "s".into(),
            turns: vec![],
        };

        registry.set_prompt("run1", prompt.clone());
        registry.start_attempt("run1", 2);

        assert_eq!(registry.prompt("run1").as_deref(), Some(&prompt));
        registry.remove("run1");
        assert_eq!(registry.prompt("run1"), None);
        assert_eq!(registry.set_prompt("run1", prompt), None);
    }
}

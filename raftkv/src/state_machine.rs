//! The replicated state machine: an ordered map that applies commands.
//!
//! It is deliberately separate from durability. Today `KvStore` logs a command
//! to the WAL and then calls `apply`. Under Raft, a command is applied only
//! after it is committed in the replicated log; the state machine itself does
//! not change.

use std::collections::BTreeMap;

use crate::command::Command;

#[derive(Debug, Default, Clone)]
pub struct StateMachine {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one command. Must be deterministic: every replica applying the
    /// same commands in the same order has to end up in the same state.
    pub fn apply(&mut self, cmd: &Command) {
        match cmd {
            Command::Put { key, value } => {
                self.data.insert(key.clone(), value.clone());
            }
            Command::Delete { key } => {
                self.data.remove(key);
            }
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.data.get(key).map(|v| v.as_slice())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_put_then_delete() {
        let mut sm = StateMachine::new();
        sm.apply(&Command::Put {
            key: b"a".to_vec(),
            value: b"1".to_vec(),
        });
        assert_eq!(sm.get(b"a"), Some(&b"1"[..]));
        assert_eq!(sm.len(), 1);

        sm.apply(&Command::Delete { key: b"a".to_vec() });
        assert_eq!(sm.get(b"a"), None);
        assert!(sm.is_empty());
    }

    #[test]
    fn deleting_a_missing_key_is_a_no_op() {
        let mut sm = StateMachine::new();
        sm.apply(&Command::Delete { key: b"x".to_vec() });
        assert!(sm.is_empty());
    }
}

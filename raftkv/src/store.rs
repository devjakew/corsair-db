//! Single-node durable key-value store (Phase 1)
//!
//! Write path: encode command -> append to WAL (fsync) -> apply to memory.
//! Log first, apply second: if crash between the two, replay on restart
//! re-applies the command, so an acknowledged write is never lost.

use std::path::Path;

use crate::command::Command;
use crate::error::Error;
use crate::state_machine::StateMachine;
use crate::wal::Wal;

pub struct KvStore {
    state: StateMachine,
    wal: Wal,
}

impl KvStore {
    /// Opens a store with durable writes (fsync on every write).
    pub fn open<P: AsRef<Path>>(path: P) -> Result<KvStore, Error> {
        Self::open_with(path, true)
    }

    /// Opens a store, choosing whether each write is fsynced. Disabling sync is
    /// useful for tests and benchmarks; it trades durability for speed.
    pub fn open_with<P: AsRef<Path>>(path: P, sync_on_write: bool) -> Result<KvStore, Error> {
        let (wal, records) = Wal::open(path, sync_on_write)?;
        let mut state = StateMachine::new();
        for payload in &records {
            let cmd = Command::decode(payload)?;
            state.apply(&cmd);
        }
        Ok(KvStore { state, wal })
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.execute(Command::Put {
            key: key.to_vec(),
            value: value.to_vec(),
        })
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<(), Error> {
        self.execute(Command::Delete { key: key.to_vec() })
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.state.get(key)
    }

    pub fn len(&self) -> usize {
        self.state.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    fn execute(&mut self, cmd: Command) -> Result<(), Error> {
        self.wal.append(&cmd.encode())?; // log first...
        self.state.apply(&cmd); // ...then apply
        Ok(())
    }
}

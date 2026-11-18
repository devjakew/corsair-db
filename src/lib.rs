//! raftkv: A Raft-replicated key-value store.
//!
//! Phase 1 : a single-node durable store built from
//! `Command` -> `Wal` -> `StateMachine`. Raft arrives later.

pub mod command;
pub mod error;
pub mod state_machine;
pub mod store;
pub mod wal;

pub use command::Command;
pub use error::Error;
pub use state_machine::StateMachine;
pub use store::KvStore;

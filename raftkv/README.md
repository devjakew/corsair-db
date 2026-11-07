# raftkv

A key-value store built from scratch in Rust, replicated across nodes with the
[Raft consensus algorithm](https://raft.github.io/raft.pdf). No Raft library:
the consensus, log, storage, and test harness are all written here.

> **Status:** Phase 1 complete (single-node durable store). Raft is next.
> See [docs/ROADMAP.md](docs/ROADMAP.md).

## Why this project

The goal is to show, with evidence, that the system stays correct when things
go wrong: crashes, torn writes, dropped and reordered messages, and network
partitions. The test harness is as much the deliverable as the store itself.

## What works today

- Durable single-node store: `PUT`, `GET`, `DELETE`
- Write-ahead log with CRC32 checksums and crash recovery (torn or corrupt
  tails are detected and truncated on open)
- A `Command` encoding and a separate `StateMachine`, so the same bytes can later
  become Raft log entries
- Standard library only, zero dependencies

## Quick start

```bash
cargo run -- kv.wal        # interactive shell
cargo test                 # unit + integration tests
```

```
> PUT language rust
OK
> GET language
rust
> DEL language
OK
```

## Architecture

```
            client
              |
   +----------v-----------+
   |        KvStore       |   Phase 1
   |  Command -> Wal ->   |
   |     StateMachine     |
   +----------------------+
```

Planned shape once Raft lands:

```
 client -> leader -> Raft log (replicated, persisted) -> commit -> StateMachine
                          ^
                          |  AppendEntries / RequestVote
                    other replicas
```

### Design principles

1. **Log first, apply second.** A write is acknowledged only after it is in
   the log (and fsynced), so a crash can never lose an acknowledged write.
2. **The state machine is deterministic and ignorant of consensus.** It just
   applies commands in order.
3. **The Raft core will be a pure state machine** driven by events (message
   received, timer fired) that returns actions (send message, persist state).
   No threads, sockets, or clocks inside it. That is what makes deterministic
   simulation testing possible.

### WAL record format

```
[ payload_len: u32 LE ][ crc32(payload): u32 LE ][ payload ... ]
```

On open, the log is read front to back. The first incomplete or
checksum-failing record ends the log and everything after it is truncated.

## Testing

| Layer | What is checked |
|---|---|
| Unit | Command encode/decode, CRC32 known-answer vector, WAL round trip |
| Integration | Reopen persistence, torn write, bit-flip corruption, mid-record truncation |
| Planned | Simulated network with partitions, drops, delays, crashes; linearizability checking |

## Results

_Benchmarks (throughput and latency for 3- and 5-node clusters) will be
published here once replication works._

## Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md) and the study notes in
[docs/RAFT_NOTES.md](docs/RAFT_NOTES.md).

## License

TODO: choose a license (MIT or Apache-2.0 are common choices).

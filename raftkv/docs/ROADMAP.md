# Roadmap

Rough pacing assumes part-time work; adjust freely. Check items off as you go.

## Phase 0: Foundations (week 1)
- [x] Repo, README skeleton, CI (fmt, clippy, tests)
- [ ] Read the Raft paper end to end; study Figure 2 (see [RAFT_NOTES.md](RAFT_NOTES.md))
- [ ] Play with the visualization at https://raft.github.io
- [ ] Skim the Raft thesis sections on log compaction and cluster membership

## Phase 1: Single-node durable store (week 1-2)
- [x] `Command` encoding
- [x] Write-ahead log with CRC32 and torn-tail recovery
- [x] `StateMachine` separated from durability
- [x] Interactive shell
- [x] Crash-recovery tests
- [ ] Run `cargo test`, `cargo fmt`, `cargo clippy` locally and push the first commit

## Phase 2: Raft core as a pure state machine (weeks 2-4)
Design: `RaftNode::step(event) -> Vec<Action>`, with no sockets, threads, or
wall-clock time inside.
- [ ] Types: `Term`, `LogIndex`, `NodeId`, `LogEntry { term, command }`
- [ ] Persistent state: `currentTerm`, `votedFor`, `log[]`
- [ ] Leader election: election timeout, `RequestVote`, vote granting rules
- [ ] Heartbeats and log replication: `AppendEntries`, `nextIndex`/`matchIndex`
- [ ] Commit rule (majority + current-term entry) and applying to the state machine
- [ ] Unit tests for each rule in Figure 2

## Phase 3: Deterministic simulation harness (weeks 4-6)
This is the part that makes the project stand out.
- [ ] Simulated clock and network (drop, delay, duplicate, reorder, partition)
- [ ] Seeded randomness so every failing run can be replayed exactly
- [ ] Crash and restart nodes (losing volatile state, keeping the persisted log)
- [ ] Invariant checks after every step (election safety, log matching,
      leader completeness, state machine safety)
- [ ] Run thousands of random schedules in CI

## Phase 4: Real networking (weeks 6-7)
- [ ] `Transport` trait; simulated and TCP implementations
- [ ] Node binary with config (id, peers, data dir)
- [ ] Client protocol with leader redirect and request deduplication
- [ ] Docker Compose file for a 3-node cluster

## Phase 5: Durability and compaction (weeks 7-8)
- [ ] Persist Raft state (term, vote, log) with fsync discipline
- [ ] Snapshots and log compaction; `InstallSnapshot`
- [ ] Group commit / batching to reduce fsync cost

## Phase 6: Correctness evidence and benchmarks (weeks 8-10)
- [ ] Record client histories and check linearizability
- [ ] Benchmarks: writes/sec, p50/p99 latency, 3 vs 5 nodes, sync vs batched
- [ ] Failover time measurements (kill the leader, time until writes succeed)
- [ ] Publish graphs and methodology in the README

## Stretch goals
- [ ] Cluster membership changes
- [ ] Read optimizations (read index or leader leases)
- [ ] Sharding across multiple Raft groups
- [ ] Web dashboard or terminal UI showing leader, terms, and log state
- [ ] Short screen recording of killing the leader and the cluster recovering

## Known limitations (be upfront about these in the README)
- The Phase 1 WAL replays the whole file on startup (fixed by snapshots in Phase 5).
- `fsync` per write limits throughput (addressed by batching in Phase 5).
- CRC32 is bitwise and unoptimized; replace with a table-driven version if profiling shows it matters.

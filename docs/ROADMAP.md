# Roadmap

Part-time project, so the week numbers are guesses. I'll move things around as I learn how long they actually take.

The order matters more than the dates: get one node solid, write Raft as a pure function, build a simulator that tries to break it, and only then add real sockets.

## Phase 0: Reading

- [x] Repo, README, CI (fmt, clippy, tests)
- [ ] CI passing on GitHub
- [ ] Read the Raft paper start to finish. Re-read Figure 2 with a pencil (notes in [RAFT_NOTES.md](RAFT_NOTES.md))
- [ ] Click through https://raft.github.io until elections and log repair make sense
- [ ] Skim the thesis chapters on log compaction and membership changes

## Phase 1: Single-node store

- [x] `Command` encoding
- [x] Write-ahead log with CRC32 and torn-tail recovery
- [x] `StateMachine` kept separate from durability
- [x] Small interactive shell
- [x] Crash recovery tests
- [x] First push

### Phase 1.5: WAL cleanup

Raft will sit on top of this file, so I want it right before moving on.

- [ ] After a failed append, refuse further writes until the log is reopened (a half-written record must never be followed by good ones)
- [ ] fsync the parent directory when creating a new log file
- [ ] Tests for both

## Phase 2: Raft core as a pure state machine

`RaftNode::step(event) -> Vec<Action>`. No sockets, threads, or real clocks inside. Time is counted in ticks and the election timeout comes from a seeded RNG.

- [ ] Types: `Term`, `LogIndex`, `NodeId`, `LogEntry { term, command }`
- [ ] Persistent state: `currentTerm`, `votedFor`, `log[]`, written through `Persist*` actions
- [ ] Rule: persist actions finish before any `Send` goes out
- [ ] Leader election: timeouts, `RequestVote`, vote granting rules
- [ ] Heartbeats and replication: `AppendEntries`, `nextIndex` / `matchIndex`
- [ ] Leader appends a no-op entry when elected (needed to commit entries from older terms)
- [ ] Commit rule (majority, and only for entries from the current term)
- [ ] Apply committed entries to the `StateMachine`
- [ ] Unit tests for each rule in Figure 2

## Phase 3: Simulator

The part I most want to get right.

- [ ] Fake clock and fake network: drop, delay, duplicate, reorder, partition
- [ ] Seeded randomness, so any failing run can be replayed exactly
- [ ] Simulated disk that throws away anything not fsynced when a node crashes
- [ ] Crash and restart nodes (volatile state lost, persisted state kept)
- [ ] Invariants checked after every step: election safety, log matching, leader completeness, state machine safety
- [ ] Record client operations and check them for linearizability (simple checker first)
- [ ] Run thousands of seeds in CI

**Good stopping point.** If I get through Phase 3, the project is already worth showing: a Raft implementation plus a simulator that checks it. I'll update the resume and README here instead of waiting for the end.

## Phase 4: Real networking

- [ ] `Transport` trait with a simulated version and a TCP version
- [ ] Node binary with config (id, peers, data directory)
- [ ] Client protocol: leader redirect, request deduplication
- [ ] Docker Compose file for a 3-node cluster

## Phase 5: Snapshots and batching

- [ ] Snapshots and log compaction
- [ ] `InstallSnapshot` for followers that fall too far behind
- [ ] Group commit to cut down on fsync cost
- [ ] Faster startup (no more replaying the whole log)

## Phase 6: Measuring it

- [ ] Run the linearizability checker against the real cluster, not just the simulator
- [ ] Benchmarks: writes per second, p50 and p99 latency, 3 nodes vs 5, synced vs batched
- [ ] Failover time: kill the leader, time how long until writes work again
- [ ] Graphs and method in the README

## Maybe later

- Membership changes
- Read optimizations (read index or leader leases)
- Several Raft groups (sharding)
- Terminal UI showing leader, terms, and log state
- A short screen recording of killing the leader and the cluster recovering

## Known limitations

- Startup replays the entire WAL until snapshots exist (Phase 5).
- One fsync per write caps throughput until batching exists (Phase 5).
- CRC32 is a bitwise implementation. Swap in a table-driven one if profiling says it matters.
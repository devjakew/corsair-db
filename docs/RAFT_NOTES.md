# Raft study notes

My own summary of the Raft paper ("In Search of an Understandable Consensus
Algorithm", Ongaro & Ousterhout). **Always check these against the paper's
Figure 2**, which is the real spec. Add your own notes and open questions as you read.

## State

**Persistent on all servers** (must survive restarts; write before responding to RPCs)
- `currentTerm`: latest term seen (starts at 0, only increases)
- `votedFor`: candidate that received our vote in the current term, or none
- `log[]`: entries, each with a command and the term when the leader received it (first index is 1)

**Volatile on all servers**
- `commitIndex`: highest log index known to be committed
- `lastApplied`: highest log index applied to the state machine

**Volatile on leaders** (reinitialized after each election)
- `nextIndex[]`: for each follower, the next log index to send
- `matchIndex[]`: for each follower, the highest index known to be replicated there

## RPCs

### AppendEntries (leader -> follower; also used as heartbeat)
Arguments: `term`, `leaderId`, `prevLogIndex`, `prevLogTerm`, `entries[]`, `leaderCommit`
Results: `term`, `success`

Receiver:
1. Reply false if `term < currentTerm`.
2. Reply false if the log has no entry at `prevLogIndex` whose term matches `prevLogTerm`.
3. If an existing entry conflicts with a new one (same index, different term), delete it and everything after it.
4. Append any new entries not already in the log.
5. If `leaderCommit > commitIndex`, set `commitIndex = min(leaderCommit, index of last new entry)`.

### RequestVote (candidate -> everyone)
Arguments: `term`, `candidateId`, `lastLogIndex`, `lastLogTerm`
Results: `term`, `voteGranted`

Receiver:
1. Reply false if `term < currentTerm`.
2. Grant the vote if `votedFor` is none or `candidateId`, **and** the candidate's log is at least as up-to-date as ours.

"Up-to-date": compare the term of the last entries; if equal, the longer log wins.

## Rules for servers

**All servers**
- If `commitIndex > lastApplied`: increment `lastApplied` and apply `log[lastApplied]` to the state machine.
- If any RPC request or response has `term > currentTerm`: set `currentTerm = term` and convert to follower.

**Followers**
- Respond to RPCs from candidates and leaders.
- If the election timeout elapses without hearing from the current leader or granting a vote: become a candidate.

**Candidates**
- On conversion: increment `currentTerm`, vote for self, reset the election timer, send `RequestVote` to all others.
- Majority of votes: become leader.
- `AppendEntries` from a new leader: become follower.
- Election timeout elapses: start a new election.

**Leaders**
- On election: send initial empty `AppendEntries` (heartbeat) to every server; repeat during idle periods.
- On a client command: append to the local log; respond after the entry is applied.
- If `last log index >= nextIndex[f]`: send `AppendEntries` with entries from `nextIndex[f]`.
  - Success: update `nextIndex[f]` and `matchIndex[f]`.
  - Failure due to log inconsistency: decrement `nextIndex[f]` and retry.
- If there is an `N > commitIndex` such that a majority of `matchIndex[i] >= N` **and** `log[N].term == currentTerm`: set `commitIndex = N`.

## Safety properties to test

| Property | Meaning |
|---|---|
| Election Safety | At most one leader per term |
| Leader Append-Only | A leader never overwrites or deletes its own log entries |
| Log Matching | If two logs have an entry with the same index and term, the logs are identical up to that index |
| Leader Completeness | A committed entry is present in the logs of all later leaders |
| State Machine Safety | No two servers apply different commands at the same index |

These become the invariant checks in the simulation harness (Phase 3).

## Things that are easy to get wrong
- Persist `currentTerm`, `votedFor`, and the log **before** replying to an RPC.
- A leader only commits entries from its **current term** by counting replicas (the `log[N].term == currentTerm` clause). Older entries are committed indirectly.
- Randomize election timeouts, or split votes can repeat forever.
- Reset the election timer when granting a vote or hearing from a legitimate leader, not on every message.
- Handle stale or reordered RPC responses (check the term and that the response still matches what you sent).
- Duplicate client requests after a retry must not be applied twice.

## Open questions
- (add yours as you read)

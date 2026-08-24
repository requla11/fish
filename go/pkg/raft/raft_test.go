package raft

import (
	"testing"
	"time"
)

func TestFollowerBecomesCandidateOnTimeout(t *testing.T) {
	node := NewRaftNode("node-1", []string{"node-2", "node-3"}, nil)
	node.SetElectionTimeout(50 * time.Millisecond)

	if node.State() != Follower {
		t.Fatal("new node must start as Follower")
	}

	// Simulate timeout
	time.Sleep(100 * time.Millisecond)
	if !node.ShouldStartElection() {
		t.Fatal("election should trigger after timeout")
	}
}

func TestLeaderElectionSingleNode(t *testing.T) {
	node := NewRaftNode("solo", nil, nil)
	won := node.StartElection()
	if !won {
		t.Fatal("single-node cluster: candidate always wins")
	}
	if !node.IsLeader() {
		t.Fatal("node must be Leader after winning")
	}
	if node.Term() != 1 {
		t.Fatalf("term should be 1, got %d", node.Term())
	}
}

func TestAppendEntriesUpdatesHeartbeat(t *testing.T) {
	follower := NewRaftNode("follower", []string{"leader"}, nil)
	_ = follower.StartElection() // become Candidate

	leader := NewRaftNode("leader", nil, nil)
	_ = leader.StartElection() // leader at term 1

	args := &AppendEntriesArgs{
		Term:         1,
		LeaderID:     "leader",
		PrevLogIndex: -1,
		PrevLogTerm:  0,
		Entries:      nil,
		LeaderCommit: -1,
	}
	reply := follower.HandleAppendEntries(args)

	if !reply.Success {
		t.Fatal("heartbeat from valid leader should succeed")
	}
	if follower.State() != Follower {
		t.Fatalf("candidate must revert to Follower on AppendEntries, got %s", follower.State())
	}
}

func TestStaleTermRejected(t *testing.T) {
	node := NewRaftNode("n", []string{"other"}, nil)
	node.StartElection() // term 1

	staleArgs := &RequestVoteArgs{Term: 0, CandidateID: "other"}
	reply := node.HandleRequestVote(staleArgs)
	if reply.VoteGranted {
		t.Fatal("vote for stale term must be rejected")
	}

	staleHeartbeat := &AppendEntriesArgs{Term: 0, LeaderID: "other"}
	hbReply := node.HandleAppendEntries(staleHeartbeat)
	if hbReply.Success {
		t.Fatal("stale heartbeat must be rejected")
	}
}

func TestHigherTermCausesStepDown(t *testing.T) {
	follower := NewRaftNode("follower", []string{"leader"}, nil)
	follower.StartElection() // becomes Leader at term 1 (single node)

	newerVote := &RequestVoteArgs{Term: 5, CandidateID: "someone"}
	reply := follower.HandleRequestVote(newerVote)
	if reply.Term != 5 {
		t.Fatalf("reply term should be 5, got %d", reply.Term)
	}
	if follower.State() != Follower {
		t.Fatal("higher-term vote must cause step down to Follower")
	}
}

func TestLogReplication(t *testing.T) {
	var applied []LogEntry
	follower := NewRaftNode("f", []string{"l"}, func(e LogEntry) {
		applied = append(applied, e)
	})

	entries := []LogEntry{
		{Term: 1, Index: 0, Command: []byte("cmd-0")},
		{Term: 1, Index: 1, Command: []byte("cmd-1")},
	}

	args := &AppendEntriesArgs{
		Term:         1,
		LeaderID:     "leader",
		PrevLogIndex: -1,
		PrevLogTerm:  0,
		Entries:      entries,
		LeaderCommit: 1,
	}
	reply := follower.HandleAppendEntries(args)

	if !reply.Success {
		t.Fatal("log replication should succeed")
	}
	if len(applied) != 2 {
		t.Fatalf("expected 2 applied entries, got %d", len(applied))
	}
	if string(applied[0].Command) != "cmd-0" {
		t.Fatalf("first command mismatch: %s", applied[0].Command)
	}
}

func TestConflictingEntriesTruncated(t *testing.T) {
	applied := []LogEntry{}
	node := NewRaftNode("n", nil, func(e LogEntry) { applied = append(applied, e) })

	// First append: 2 entries at term 1.
	batch1 := &AppendEntriesArgs{
		Term: 1, LeaderID: "l",
		PrevLogIndex: -1, PrevLogTerm: 0,
		Entries: []LogEntry{
			{Term: 1, Index: 0, Command: []byte("a")},
			{Term: 1, Index: 1, Command: []byte("b")},
		},
		LeaderCommit: -1,
	}
	node.HandleAppendEntries(batch1)

	// Conflicting append: overwrite index 1 with a different entry.
	batch2 := &AppendEntriesArgs{
		Term: 2, LeaderID: "l2",
		PrevLogIndex: 0, PrevLogTerm: 1,
		Entries: []LogEntry{
			{Term: 2, Index: 1, Command: []byte("conflict")},
		},
		LeaderCommit: -1,
	}
	reply := node.HandleAppendEntries(batch2)
	if !reply.Success {
		t.Fatal("conflicting append should succeed after truncation")
	}
	if len(node.log) != 2 {
		t.Fatalf("log should have 2 entries after truncation+append, got %d", len(node.log))
	}
	if string(node.log[1].Command) != "conflict" {
		t.Fatal("entry at index 1 should be the conflicting entry")
	}
}

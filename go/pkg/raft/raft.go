package raft

import (
	"sync"
	"time"
)

// NodeState represents the Raft consensus state of a node.
type NodeState int

const (
	Follower NodeState = iota
	Candidate
	Leader
)

func (s NodeState) String() string {
	switch s {
	case Follower:
		return "Follower"
	case Candidate:
		return "Candidate"
	case Leader:
		return "Leader"
	default:
		return "Unknown"
	}
}

// LogEntry is a single entry in the replicated log.
type LogEntry struct {
	Term    int
	Index   int
	Command []byte
}

// RPC arguments and responses for RequestVote.
type RequestVoteArgs struct {
	Term         int
	CandidateID  string
	LastLogIndex int
	LastLogTerm  int
}

type RequestVoteReply struct {
	Term        int
	VoteGranted bool
}

// RPC arguments and responses for AppendEntries (heartbeats + log replication).
type AppendEntriesArgs struct {
	Term         int
	LeaderID     string
	PrevLogIndex int
	PrevLogTerm  int
	Entries      []LogEntry
	LeaderCommit int
}

type AppendEntriesReply struct {
	Term    int
	Success bool
}

// RaftNode is a single participant in the Raft consensus group.
type RaftNode struct {
	mu sync.Mutex

	id       string
	state    NodeState
	peers    []string
	term     int
	votedFor string
	log      []LogEntry

	commitIndex int
	lastApplied int

	// Leader state: next index to send to each follower.
	nextIndex  map[string]int
	matchIndex map[string]int

	// Timing.
	electionTimeout time.Duration
	lastHeartbeat   time.Time

	// Callbacks for applying committed entries.
	applier func(entry LogEntry)
}

// NewRaftNode creates a Raft node in Follower state.
func NewRaftNode(id string, peers []string, applier func(LogEntry)) *RaftNode {
	return &RaftNode{
		id:              id,
		state:           Follower,
		peers:           peers,
		term:            0,
		votedFor:        "",
		commitIndex:     -1,
		lastApplied:     -1,
		nextIndex:       make(map[string]int),
		matchIndex:      make(map[string]int),
		electionTimeout: randomElectionTimeout(),
		lastHeartbeat:   time.Now(),
		applier:         applier,
	}
}

func randomElectionTimeout() time.Duration {
	// Raft spec: 150-300ms randomized to prevent split votes.
	n := time.Now().UnixNano() % int64(150)
	return time.Duration(150+int(n)) * time.Millisecond
}

// SetElectionTimeout overrides the randomized timeout (for tests).
func (r *RaftNode) SetElectionTimeout(d time.Duration) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.electionTimeout = d
}

// IsLeader returns whether this node is currently the leader.
func (r *RaftNode) IsLeader() bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.state == Leader
}

// LeaderID returns the current leader's ID if known.
func (r *RaftNode) LeaderID() string {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.state == Leader {
		return r.id
	}
	return ""
}

// Term returns the current term number.
func (r *RaftNode) Term() int {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.term
}

// State returns the current node state.
func (r *RaftNode) State() NodeState {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.state
}

// HandleRequestVote processes an incoming RequestVote RPC.
func (r *RaftNode) HandleRequestVote(args *RequestVoteArgs) *RequestVoteReply {
	r.mu.Lock()
	defer r.mu.Unlock()

	if args.Term > r.term {
		r.term = args.Term
		r.state = Follower
		r.votedFor = ""
	}
	reply := &RequestVoteReply{Term: r.term, VoteGranted: false}

	if args.Term < r.term {
		return reply
	}

	if r.votedFor == "" || r.votedFor == args.CandidateID {
		lastLogIdx := r.lastLogIndex()
		lastLogTerm := r.lastLogTerm()
		if args.LastLogIndex >= lastLogIdx && args.LastLogTerm >= lastLogTerm {
			r.votedFor = args.CandidateID
			reply.VoteGranted = true
			r.lastHeartbeat = time.Now()
		}
	}
	return reply
}

// HandleAppendEntries processes an incoming AppendEntries RPC (heartbeat).
func (r *RaftNode) HandleAppendEntries(args *AppendEntriesArgs) *AppendEntriesReply {
	r.mu.Lock()
	defer r.mu.Unlock()

	reply := &AppendEntriesReply{Term: r.term, Success: false}

	if args.Term < r.term {
		return reply
	}
	if args.Term > r.term {
		r.term = args.Term
		r.state = Follower
		r.votedFor = ""
	}

	if r.state != Follower {
		r.state = Follower
	}
	r.lastHeartbeat = time.Now()

	// Check log consistency.
	lastLogIdx := r.lastLogIndex()
	if args.PrevLogIndex > lastLogIdx {
		return reply
	}
	if args.PrevLogIndex >= 0 && r.log[args.PrevLogIndex].Term != args.PrevLogTerm {
		return reply
	}

	// Append new entries.
	for i, entry := range args.Entries {
		idx := args.PrevLogIndex + 1 + i
		if idx < len(r.log) && r.log[idx].Term != entry.Term {
			r.log = r.log[:idx] // Truncate conflicting entries
		}
		if idx >= len(r.log) {
			r.log = append(r.log, entry)
		}
	}

	// Advance commit index.
	if args.LeaderCommit > r.commitIndex {
		newCommit := args.LeaderCommit
		if newCommit > r.lastLogIndex() {
			newCommit = r.lastLogIndex()
		}
		r.commitIndex = newCommit
		r.applyCommitted()
	}

	reply.Success = true
	return reply
}

// StartElection transitions to Candidate and requests votes from all peers.
// Returns true if this node won the election (became Leader).
func (r *RaftNode) StartElection() bool {
	r.mu.Lock()
	r.state = Candidate
	r.term++
	currentTerm := r.term
	r.votedFor = r.id // Vote for self
	r.lastHeartbeat = time.Now()

	args := &RequestVoteArgs{
		Term:         currentTerm,
		CandidateID:  r.id,
		LastLogIndex: r.lastLogIndex(),
		LastLogTerm:  r.lastLogTerm(),
	}
	r.mu.Unlock()

	votes := 1 // Self-vote
	majority := len(r.peers)/2 + 1
	if votes >= majority {
		r.becomeLeader()
		return true
	}

	// In a real implementation we'd send RPCs here. For the coordinator,
	// each node calls HandleRequestVote on its peers directly. The caller
	// is responsible for collecting vote replies.
	_ = args
	return votes >= majority
}

// Tick advances the election timer. If the timeout has elapsed without
// receiving a heartbeat, triggers an election.
func (r *RaftNode) ShouldStartElection() bool {
	r.mu.Lock()
	defer r.mu.Unlock()

	if r.state == Leader {
		return false
	}
	return time.Since(r.lastHeartbeat) > r.electionTimeout
}

// SendHeartbeats is called by the leader to maintain authority.
func (r *RaftNode) HeartbeatArgs() *AppendEntriesArgs {
	r.mu.Lock()
	defer r.mu.Unlock()

	if r.state != Leader {
		return nil
	}
	return &AppendEntriesArgs{
		Term:         r.term,
		LeaderID:     r.id,
		PrevLogIndex: r.lastLogIndex(),
		PrevLogTerm:  r.lastLogTerm(),
		Entries:      nil,
		LeaderCommit: r.commitIndex,
	}
}

func (r *RaftNode) becomeLeader() {
	r.mu.Lock()
	r.state = Leader
	for _, peer := range r.peers {
		r.nextIndex[peer] = r.lastLogIndex() + 1
		r.matchIndex[peer] = 0
	}
	r.mu.Unlock()
}

func (r *RaftNode) lastLogIndex() int {
	return len(r.log) - 1
}

func (r *RaftNode) lastLogTerm() int {
	if len(r.log) == 0 {
		return 0
	}
	return r.log[len(r.log)-1].Term
}

func (r *RaftNode) applyCommitted() {
	for r.lastApplied < r.commitIndex && r.applier != nil {
		r.lastApplied++
		if r.lastApplied < len(r.log) {
			r.applier(r.log[r.lastApplied])
		}
	}
}

package fishv1

import (
	"bytes"
	"errors"
)

type WorkerRegistration struct {
	WorkerID            string            `json:"worker_id"`
	Address             string            `json:"address"`
	CPUCores            int32             `json:"cpu_cores"`
	MemoryBytes         int64             `json:"memory_bytes"`
	SupportedToolchains []string          `json:"supported_toolchains"`
	Tags                map[string]string `json:"tags"`
}

func (w *WorkerRegistration) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, w.WorkerID, &buf)
	encodeStringField(2, w.Address, &buf)
	encodeInt32Field(3, w.CPUCores, &buf)
	encodeInt64Field(4, w.MemoryBytes, &buf)
	encodeRepeatedString(5, w.SupportedToolchains, &buf)
	if len(w.Tags) > 0 {
		encodeMapStringString(6, w.Tags, &buf)
	}
	return buf.Bytes()
}

func (w *WorkerRegistration) Decode(buf []byte) error {
	w.Tags = make(map[string]string)
	w.SupportedToolchains = nil
	offset := 0
	for offset < len(buf) {
		tag, err := decodeVarint(buf, &offset)
		if err != nil {
			return err
		}
		fnum := uint32(tag >> 3)
		wtype := byte(tag & 0x07)
		switch fnum {
		case 1:
			w.WorkerID, err = decodeStringField(buf, &offset)
		case 2:
			w.Address, err = decodeStringField(buf, &offset)
		case 3:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			w.CPUCores = int32(v)
		case 4:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			w.MemoryBytes = int64(v)
		case 5:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				w.SupportedToolchains = append(w.SupportedToolchains, s)
			}
		case 6:
			err = decodeMapStringStringEntry(buf, &offset, w.Tags)
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

type WorkerHeartbeat struct {
	WorkerID             string  `json:"worker_id"`
	CPULoad              float64 `json:"cpu_load"`
	AvailableMemoryBytes int64   `json:"available_memory_bytes"`
	ActiveJobs           int32   `json:"active_jobs"`
	Timestamp            int64   `json:"timestamp"`
}

func (h *WorkerHeartbeat) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, h.WorkerID, &buf)
	encodeDoubleField(2, h.CPULoad, &buf)
	encodeInt64Field(3, h.AvailableMemoryBytes, &buf)
	encodeInt32Field(4, h.ActiveJobs, &buf)
	encodeInt64Field(5, h.Timestamp, &buf)
	return buf.Bytes()
}

func (h *WorkerHeartbeat) Decode(buf []byte) error {
	offset := 0
	for offset < len(buf) {
		tag, err := decodeVarint(buf, &offset)
		if err != nil {
			return err
		}
		fnum := uint32(tag >> 3)
		wtype := byte(tag & 0x07)
		switch fnum {
		case 1:
			h.WorkerID, err = decodeStringField(buf, &offset)
		case 2:
			h.CPULoad, err = decodeDoubleField(buf, &offset)
		case 3:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			h.AvailableMemoryBytes = int64(v)
		case 4:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			h.ActiveJobs = int32(v)
		case 5:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			h.Timestamp = int64(v)
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

type HeartbeatAck struct {
	Accepted                 bool  `json:"accepted"`
	NextHeartbeatIntervalMs  int64 `json:"next_heartbeat_interval_ms"`
}

func (a *HeartbeatAck) Encode() []byte {
	var buf bytes.Buffer
	encodeBoolField(1, a.Accepted, &buf)
	encodeInt64Field(2, a.NextHeartbeatIntervalMs, &buf)
	return buf.Bytes()
}

func (a *HeartbeatAck) Decode(buf []byte) error {
	offset := 0
	for offset < len(buf) {
		tag, err := decodeVarint(buf, &offset)
		if err != nil {
			return err
		}
		fnum := uint32(tag >> 3)
		wtype := byte(tag & 0x07)
		switch fnum {
		case 1:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			a.Accepted = (v != 0)
		case 2:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			a.NextHeartbeatIntervalMs = int64(v)
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

type TaskAssignment struct {
	JobID string     `json:"job_id"`
	Task  *BuildTask `json:"task"`
}

func (a *TaskAssignment) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, a.JobID, &buf)
	if a.Task != nil {
		tb := a.Task.Encode()
		encodeTag(2, 2, &buf)
		encodeVarint(uint64(len(tb)), &buf)
		buf.Write(tb)
	}
	return buf.Bytes()
}

func (a *TaskAssignment) Decode(buf []byte) error {
	a.Task = nil
	offset := 0
	for offset < len(buf) {
		tag, err := decodeVarint(buf, &offset)
		if err != nil {
			return err
		}
		fnum := uint32(tag >> 3)
		wtype := byte(tag & 0x07)
		switch fnum {
		case 1:
			a.JobID, err = decodeStringField(buf, &offset)
		case 2:
			var l uint64
			l, err = decodeVarint(buf, &offset)
			if err == nil {
				length := int(l)
				if offset+length > len(buf) {
					return errors.New("unexpected EOF in assigned task")
				}
				var t BuildTask
				if err = t.Decode(buf[offset : offset+length]); err == nil {
					a.Task = &t
				}
				offset += length
			}
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

type TaskStatusUpdate struct {
	JobID  string      `json:"job_id"`
	TaskID string      `json:"task_id"`
	State  string      `json:"state"`
	Result *TaskResult `json:"result"`
}

func (u *TaskStatusUpdate) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, u.JobID, &buf)
	encodeStringField(2, u.TaskID, &buf)
	encodeStringField(3, u.State, &buf)
	if u.Result != nil {
		rb := u.Result.Encode()
		encodeTag(4, 2, &buf)
		encodeVarint(uint64(len(rb)), &buf)
		buf.Write(rb)
	}
	return buf.Bytes()
}

func (u *TaskStatusUpdate) Decode(buf []byte) error {
	u.Result = nil
	offset := 0
	for offset < len(buf) {
		tag, err := decodeVarint(buf, &offset)
		if err != nil {
			return err
		}
		fnum := uint32(tag >> 3)
		wtype := byte(tag & 0x07)
		switch fnum {
		case 1:
			u.JobID, err = decodeStringField(buf, &offset)
		case 2:
			u.TaskID, err = decodeStringField(buf, &offset)
		case 3:
			u.State, err = decodeStringField(buf, &offset)
		case 4:
			var l uint64
			l, err = decodeVarint(buf, &offset)
			if err == nil {
				length := int(l)
				if offset+length > len(buf) {
					return errors.New("unexpected EOF in task result")
				}
				var r TaskResult
				if err = r.Decode(buf[offset : offset+length]); err == nil {
					u.Result = &r
				}
				offset += length
			}
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

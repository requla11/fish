package fishv1

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"math"
	"sort"
)

func encodeVarint(val uint64, buf *bytes.Buffer) {
	for val >= 0x80 {
		buf.WriteByte(byte(val&0x7F) | 0x80)
		val >>= 7
	}
	buf.WriteByte(byte(val))
}

func decodeVarint(buf []byte, offset *int) (uint64, error) {
	var result uint64
	var shift uint
	for *offset < len(buf) {
		b := buf[*offset]
		*offset++
		result |= uint64(b&0x7F) << shift
		if (b & 0x80) == 0 {
			return result, nil
		}
		shift += 7
		if shift >= 64 {
			return 0, errors.New("varint overflow")
		}
	}
	return 0, errors.New("unexpected EOF reading varint")
}

func encodeTag(fieldNum uint32, wireType byte, buf *bytes.Buffer) {
	tag := (uint64(fieldNum) << 3) | uint64(wireType)
	encodeVarint(tag, buf)
}

func skipField(wireType byte, buf []byte, offset *int) error {
	switch wireType {
	case 0:
		_, err := decodeVarint(buf, offset)
		return err
	case 1:
		if *offset+8 > len(buf) {
			return errors.New("unexpected EOF for 64-bit field")
		}
		*offset += 8
		return nil
	case 2:
		l, err := decodeVarint(buf, offset)
		if err != nil {
			return err
		}
		if *offset+int(l) > len(buf) {
			return errors.New("unexpected EOF for length-delimited field")
		}
		*offset += int(l)
		return nil
	case 5:
		if *offset+4 > len(buf) {
			return errors.New("unexpected EOF for 32-bit field")
		}
		*offset += 4
		return nil
	default:
		return fmt.Errorf("unsupported wire type: %d", wireType)
	}
}

func encodeStringField(fieldNum uint32, val string, buf *bytes.Buffer) {
	if len(val) > 0 {
		encodeTag(fieldNum, 2, buf)
		encodeVarint(uint64(len(val)), buf)
		buf.WriteString(val)
	}
}

func decodeStringField(buf []byte, offset *int) (string, error) {
	l, err := decodeVarint(buf, offset)
	if err != nil {
		return "", err
	}
	length := int(l)
	if *offset+length > len(buf) {
		return "", errors.New("unexpected EOF reading string")
	}
	s := string(buf[*offset : *offset+length])
	*offset += length
	return s, nil
}

func encodeInt32Field(fieldNum uint32, val int32, buf *bytes.Buffer) {
	if val != 0 {
		encodeTag(fieldNum, 0, buf)
		encodeVarint(uint64(val), buf)
	}
}

func encodeInt64Field(fieldNum uint32, val int64, buf *bytes.Buffer) {
	if val != 0 {
		encodeTag(fieldNum, 0, buf)
		encodeVarint(uint64(val), buf)
	}
}

func encodeBoolField(fieldNum uint32, val bool, buf *bytes.Buffer) {
	if val {
		encodeTag(fieldNum, 0, buf)
		encodeVarint(1, buf)
	}
}

func encodeDoubleField(fieldNum uint32, val float64, buf *bytes.Buffer) {
	if val != 0 {
		encodeTag(fieldNum, 1, buf)
		var b [8]byte
		binary.LittleEndian.PutUint64(b[:], math.Float64bits(val))
		buf.Write(b[:])
	}
}

func decodeDoubleField(buf []byte, offset *int) (float64, error) {
	if *offset+8 > len(buf) {
		return 0, errors.New("unexpected EOF reading double")
	}
	bits := binary.LittleEndian.Uint64(buf[*offset : *offset+8])
	*offset += 8
	return math.Float64frombits(bits), nil
}

func encodeRepeatedString(fieldNum uint32, vals []string, buf *bytes.Buffer) {
	for _, v := range vals {
		encodeStringField(fieldNum, v, buf)
	}
}

func encodeMapStringString(fieldNum uint32, m map[string]string, buf *bytes.Buffer) {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		v := m[k]
		var entryBuf bytes.Buffer
		encodeStringField(1, k, &entryBuf)
		encodeStringField(2, v, &entryBuf)
		encodeTag(fieldNum, 2, buf)
		encodeVarint(uint64(entryBuf.Len()), buf)
		buf.Write(entryBuf.Bytes())
	}
}

func decodeMapStringStringEntry(buf []byte, offset *int, m map[string]string) error {
	l, err := decodeVarint(buf, offset)
	if err != nil {
		return err
	}
	length := int(l)
	if *offset+length > len(buf) {
		return errors.New("unexpected EOF in map entry")
	}
	end := *offset + length
	var k, v string
	for *offset < end {
		tag, err := decodeVarint(buf, offset)
		if err != nil {
			return err
		}
		fnum := uint32(tag >> 3)
		wtype := byte(tag & 0x07)
		switch fnum {
		case 1:
			if wtype == 2 {
				k, err = decodeStringField(buf, offset)
				if err != nil {
					return err
				}
			}
		case 2:
			if wtype == 2 {
				v, err = decodeStringField(buf, offset)
				if err != nil {
					return err
				}
			}
		default:
			if err := skipField(wtype, buf, offset); err != nil {
				return err
			}
		}
	}
	m[k] = v
	return nil
}

type BuildTask struct {
	ID           string            `json:"id"`
	PackageName  string            `json:"package_name"`
	Toolchain    string            `json:"toolchain"`
	Command      string            `json:"command"`
	Args         []string          `json:"args"`
	Inputs       []string          `json:"inputs"`
	Outputs      []string          `json:"outputs"`
	Dependencies []string          `json:"dependencies"`
	Env          map[string]string `json:"env"`
	TimeoutMs    int64             `json:"timeout_ms"`
}

func (t *BuildTask) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, t.ID, &buf)
	encodeStringField(2, t.PackageName, &buf)
	encodeStringField(3, t.Toolchain, &buf)
	encodeStringField(4, t.Command, &buf)
	encodeRepeatedString(5, t.Args, &buf)
	encodeRepeatedString(6, t.Inputs, &buf)
	encodeRepeatedString(7, t.Outputs, &buf)
	encodeRepeatedString(8, t.Dependencies, &buf)
	if len(t.Env) > 0 {
		encodeMapStringString(9, t.Env, &buf)
	}
	encodeInt64Field(10, t.TimeoutMs, &buf)
	return buf.Bytes()
}

func (t *BuildTask) Decode(buf []byte) error {
	t.Env = make(map[string]string)
	t.Args = nil
	t.Inputs = nil
	t.Outputs = nil
	t.Dependencies = nil
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
			t.ID, err = decodeStringField(buf, &offset)
		case 2:
			t.PackageName, err = decodeStringField(buf, &offset)
		case 3:
			t.Toolchain, err = decodeStringField(buf, &offset)
		case 4:
			t.Command, err = decodeStringField(buf, &offset)
		case 5:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				t.Args = append(t.Args, s)
			}
		case 6:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				t.Inputs = append(t.Inputs, s)
			}
		case 7:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				t.Outputs = append(t.Outputs, s)
			}
		case 8:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				t.Dependencies = append(t.Dependencies, s)
			}
		case 9:
			err = decodeMapStringStringEntry(buf, &offset, t.Env)
		case 10:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			t.TimeoutMs = int64(v)
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

type TaskResult struct {
	TaskID        string            `json:"task_id"`
	ExitCode      int32             `json:"exit_code"`
	Stdout        string            `json:"stdout"`
	Stderr        string            `json:"stderr"`
	DurationMs    int64             `json:"duration_ms"`
	Cached        bool              `json:"cached"`
	Fingerprint   string            `json:"fingerprint"`
	OutputDigests map[string]string `json:"output_digests"`
}

func (r *TaskResult) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, r.TaskID, &buf)
	encodeInt32Field(2, r.ExitCode, &buf)
	encodeStringField(3, r.Stdout, &buf)
	encodeStringField(4, r.Stderr, &buf)
	encodeInt64Field(5, r.DurationMs, &buf)
	encodeBoolField(6, r.Cached, &buf)
	encodeStringField(7, r.Fingerprint, &buf)
	if len(r.OutputDigests) > 0 {
		encodeMapStringString(8, r.OutputDigests, &buf)
	}
	return buf.Bytes()
}

func (r *TaskResult) Decode(buf []byte) error {
	r.OutputDigests = make(map[string]string)
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
			r.TaskID, err = decodeStringField(buf, &offset)
		case 2:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			r.ExitCode = int32(v)
		case 3:
			r.Stdout, err = decodeStringField(buf, &offset)
		case 4:
			r.Stderr, err = decodeStringField(buf, &offset)
		case 5:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			r.DurationMs = int64(v)
		case 6:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			r.Cached = (v != 0)
		case 7:
			r.Fingerprint, err = decodeStringField(buf, &offset)
		case 8:
			err = decodeMapStringStringEntry(buf, &offset, r.OutputDigests)
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

type BuildGraph struct {
	RootPackage    string      `json:"root_package"`
	Tasks          []BuildTask `json:"tasks"`
	ExecutionOrder []string    `json:"execution_order"`
}

func (g *BuildGraph) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, g.RootPackage, &buf)
	for _, t := range g.Tasks {
		tb := t.Encode()
		encodeTag(2, 2, &buf)
		encodeVarint(uint64(len(tb)), &buf)
		buf.Write(tb)
	}
	encodeRepeatedString(3, g.ExecutionOrder, &buf)
	return buf.Bytes()
}

func (g *BuildGraph) Decode(buf []byte) error {
	g.Tasks = nil
	g.ExecutionOrder = nil
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
			g.RootPackage, err = decodeStringField(buf, &offset)
		case 2:
			var l uint64
			l, err = decodeVarint(buf, &offset)
			if err == nil {
				length := int(l)
				if offset+length > len(buf) {
					return errors.New("unexpected EOF in task")
				}
				var task BuildTask
				if err = task.Decode(buf[offset : offset+length]); err == nil {
					g.Tasks = append(g.Tasks, task)
				}
				offset += length
			}
		case 3:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				g.ExecutionOrder = append(g.ExecutionOrder, s)
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

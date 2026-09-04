package fishv1

import (
	"bytes"
)

type FailureAnalysisRequest struct {
	TaskID    string `json:"task_id"`
	Toolchain string `json:"toolchain"`
	Command   string `json:"command"`
	Stderr    string `json:"stderr"`
	Stdout    string `json:"stdout"`
	ExitCode  int32  `json:"exit_code"`
}

func (r *FailureAnalysisRequest) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, r.TaskID, &buf)
	encodeStringField(2, r.Toolchain, &buf)
	encodeStringField(3, r.Command, &buf)
	encodeStringField(4, r.Stderr, &buf)
	encodeStringField(5, r.Stdout, &buf)
	encodeInt32Field(6, r.ExitCode, &buf)
	return buf.Bytes()
}

func (r *FailureAnalysisRequest) Decode(buf []byte) error {
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
			r.Toolchain, err = decodeStringField(buf, &offset)
		case 3:
			r.Command, err = decodeStringField(buf, &offset)
		case 4:
			r.Stderr, err = decodeStringField(buf, &offset)
		case 5:
			r.Stdout, err = decodeStringField(buf, &offset)
		case 6:
			var v uint64
			v, err = decodeVarint(buf, &offset)
			r.ExitCode = int32(v)
		default:
			err = skipField(wtype, buf, &offset)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

type FailureAnalysisResponse struct {
	ErrorCategory  string   `json:"error_category"`
	RootCause      string   `json:"root_cause"`
	Confidence     float64  `json:"confidence"`
	SuggestedFixes []string `json:"suggested_fixes"`
	AffectedFiles  []string `json:"affected_files"`
}

func (r *FailureAnalysisResponse) Encode() []byte {
	var buf bytes.Buffer
	encodeStringField(1, r.ErrorCategory, &buf)
	encodeStringField(2, r.RootCause, &buf)
	encodeDoubleField(3, r.Confidence, &buf)
	encodeRepeatedString(4, r.SuggestedFixes, &buf)
	encodeRepeatedString(5, r.AffectedFiles, &buf)
	return buf.Bytes()
}

func (r *FailureAnalysisResponse) Decode(buf []byte) error {
	r.SuggestedFixes = nil
	r.AffectedFiles = nil
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
			r.ErrorCategory, err = decodeStringField(buf, &offset)
		case 2:
			r.RootCause, err = decodeStringField(buf, &offset)
		case 3:
			r.Confidence, err = decodeDoubleField(buf, &offset)
		case 4:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				r.SuggestedFixes = append(r.SuggestedFixes, s)
			}
		case 5:
			var s string
			s, err = decodeStringField(buf, &offset)
			if err == nil {
				r.AffectedFiles = append(r.AffectedFiles, s)
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

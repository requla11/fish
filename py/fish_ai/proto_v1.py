import struct
from dataclasses import dataclass, field
from typing import List, Dict, Optional

def encode_varint(val: int) -> bytes:
    buf = bytearray()
    while val >= 0x80:
        buf.append((val & 0x7F) | 0x80)
        val >>= 7
    buf.append(val & 0x7F)
    return bytes(buf)

def decode_varint(data: bytes, offset: int = 0):
    result = 0
    shift = 0
    while offset < len(data):
        b = data[offset]
        offset += 1
        result |= (b & 0x7F) << shift
        if not (b & 0x80):
            return result, offset
        shift += 7
        if shift >= 64:
            raise ValueError("varint overflow")
    raise ValueError("unexpected EOF reading varint")

def encode_tag(field_num: int, wire_type: int) -> bytes:
    return encode_varint((field_num << 3) | wire_type)

def skip_field(data: bytes, offset: int, wire_type: int) -> int:
    if wire_type == 0:
        _, offset = decode_varint(data, offset)
        return offset
    elif wire_type == 1:
        if offset + 8 > len(data):
            raise ValueError("unexpected EOF for 64-bit field")
        return offset + 8
    elif wire_type == 2:
        length, offset = decode_varint(data, offset)
        if offset + length > len(data):
            raise ValueError("unexpected EOF for length-delimited field")
        return offset + length
    elif wire_type == 5:
        if offset + 4 > len(data):
            raise ValueError("unexpected EOF for 32-bit field")
        return offset + 4
    else:
        raise ValueError(f"unsupported wire type: {wire_type}")

def encode_string_field(field_num: int, val: str) -> bytes:
    if not val:
        return b""
    encoded = val.encode("utf-8")
    return encode_tag(field_num, 2) + encode_varint(len(encoded)) + encoded

def decode_string_field(data: bytes, offset: int):
    length, offset = decode_varint(data, offset)
    if offset + length > len(data):
        raise ValueError("unexpected EOF reading string")
    s = data[offset:offset+length].decode("utf-8")
    return s, offset + length

def encode_int32_field(field_num: int, val: int) -> bytes:
    if val == 0:
        return b""
    return encode_tag(field_num, 0) + encode_varint(val & 0xFFFFFFFF)

def encode_int64_field(field_num: int, val: int) -> bytes:
    if val == 0:
        return b""
    return encode_tag(field_num, 0) + encode_varint(val)

def encode_bool_field(field_num: int, val: bool) -> bytes:
    if not val:
        return b""
    return encode_tag(field_num, 0) + encode_varint(1)

def encode_double_field(field_num: int, val: float) -> bytes:
    if val == 0.0:
        return b""
    return encode_tag(field_num, 1) + struct.pack("<d", val)

def decode_double_field(data: bytes, offset: int):
    if offset + 8 > len(data):
        raise ValueError("unexpected EOF reading double")
    val = struct.unpack("<d", data[offset:offset+8])[0]
    return val, offset + 8

def encode_repeated_string(field_num: int, vals: List[str]) -> bytes:
    buf = bytearray()
    for v in vals:
        buf.extend(encode_string_field(field_num, v))
    return bytes(buf)

def encode_map_string_string(field_num: int, m: Dict[str, str]) -> bytes:
    buf = bytearray()
    for k in sorted(m.keys()):
        v = m[k]
        entry = encode_string_field(1, k) + encode_string_field(2, v)
        buf.extend(encode_tag(field_num, 2) + encode_varint(len(entry)) + entry)
    return bytes(buf)

def decode_map_string_string_entry(data: bytes, offset: int, m: Dict[str, str]) -> int:
    length, offset = decode_varint(data, offset)
    if offset + length > len(data):
        raise ValueError("unexpected EOF in map entry")
    end = offset + length
    k, v = "", ""
    while offset < end:
        tag, offset = decode_varint(data, offset)
        fnum = tag >> 3
        wtype = tag & 0x07
        if fnum == 1 and wtype == 2:
            k, offset = decode_string_field(data, offset)
        elif fnum == 2 and wtype == 2:
            v, offset = decode_string_field(data, offset)
        else:
            offset = skip_field(data, offset, wtype)
    m[k] = v
    return offset

@dataclass
class BuildTask:
    id: str = ""
    package_name: str = ""
    toolchain: str = ""
    command: str = ""
    args: List[str] = field(default_factory=list)
    inputs: List[str] = field(default_factory=list)
    outputs: List[str] = field(default_factory=list)
    dependencies: List[str] = field(default_factory=list)
    env: Dict[str, str] = field(default_factory=dict)
    timeout_ms: int = 0

    def encode(self) -> bytes:
        buf = bytearray()
        buf.extend(encode_string_field(1, self.id))
        buf.extend(encode_string_field(2, self.package_name))
        buf.extend(encode_string_field(3, self.toolchain))
        buf.extend(encode_string_field(4, self.command))
        buf.extend(encode_repeated_string(5, self.args))
        buf.extend(encode_repeated_string(6, self.inputs))
        buf.extend(encode_repeated_string(7, self.outputs))
        buf.extend(encode_repeated_string(8, self.dependencies))
        if self.env:
            buf.extend(encode_map_string_string(9, self.env))
        buf.extend(encode_int64_field(10, self.timeout_ms))
        return bytes(buf)

    @classmethod
    def decode(cls, data: bytes):
        obj = cls()
        offset = 0
        while offset < len(data):
            tag, offset = decode_varint(data, offset)
            fnum = tag >> 3
            wtype = tag & 0x07
            if fnum == 1 and wtype == 2:
                obj.id, offset = decode_string_field(data, offset)
            elif fnum == 2 and wtype == 2:
                obj.package_name, offset = decode_string_field(data, offset)
            elif fnum == 3 and wtype == 2:
                obj.toolchain, offset = decode_string_field(data, offset)
            elif fnum == 4 and wtype == 2:
                obj.command, offset = decode_string_field(data, offset)
            elif fnum == 5 and wtype == 2:
                s, offset = decode_string_field(data, offset)
                obj.args.append(s)
            elif fnum == 6 and wtype == 2:
                s, offset = decode_string_field(data, offset)
                obj.inputs.append(s)
            elif fnum == 7 and wtype == 2:
                s, offset = decode_string_field(data, offset)
                obj.outputs.append(s)
            elif fnum == 8 and wtype == 2:
                s, offset = decode_string_field(data, offset)
                obj.dependencies.append(s)
            elif fnum == 9 and wtype == 2:
                offset = decode_map_string_string_entry(data, offset, obj.env)
            elif fnum == 10 and wtype == 0:
                obj.timeout_ms, offset = decode_varint(data, offset)
            else:
                offset = skip_field(data, offset, wtype)
        return obj

@dataclass
class TaskResult:
    task_id: str = ""
    exit_code: int = 0
    stdout: str = ""
    stderr: str = ""
    duration_ms: int = 0
    cached: bool = False
    fingerprint: str = ""
    output_digests: Dict[str, str] = field(default_factory=dict)

    def encode(self) -> bytes:
        buf = bytearray()
        buf.extend(encode_string_field(1, self.task_id))
        buf.extend(encode_int32_field(2, self.exit_code))
        buf.extend(encode_string_field(3, self.stdout))
        buf.extend(encode_string_field(4, self.stderr))
        buf.extend(encode_int64_field(5, self.duration_ms))
        buf.extend(encode_bool_field(6, self.cached))
        buf.extend(encode_string_field(7, self.fingerprint))
        if self.output_digests:
            buf.extend(encode_map_string_string(8, self.output_digests))
        return bytes(buf)

    @classmethod
    def decode(cls, data: bytes):
        obj = cls()
        offset = 0
        while offset < len(data):
            tag, offset = decode_varint(data, offset)
            fnum = tag >> 3
            wtype = tag & 0x07
            if fnum == 1 and wtype == 2:
                obj.task_id, offset = decode_string_field(data, offset)
            elif fnum == 2 and wtype == 0:
                obj.exit_code, offset = decode_varint(data, offset)
            elif fnum == 3 and wtype == 2:
                obj.stdout, offset = decode_string_field(data, offset)
            elif fnum == 4 and wtype == 2:
                obj.stderr, offset = decode_string_field(data, offset)
            elif fnum == 5 and wtype == 0:
                obj.duration_ms, offset = decode_varint(data, offset)
            elif fnum == 6 and wtype == 0:
                v, offset = decode_varint(data, offset)
                obj.cached = bool(v)
            elif fnum == 7 and wtype == 2:
                obj.fingerprint, offset = decode_string_field(data, offset)
            elif fnum == 8 and wtype == 2:
                offset = decode_map_string_string_entry(data, offset, obj.output_digests)
            else:
                offset = skip_field(data, offset, wtype)
        return obj

@dataclass
class FailureAnalysisRequest:
    task_id: str = ""
    toolchain: str = ""
    command: str = ""
    stderr: str = ""
    stdout: str = ""
    exit_code: int = 0

    def encode(self) -> bytes:
        buf = bytearray()
        buf.extend(encode_string_field(1, self.task_id))
        buf.extend(encode_string_field(2, self.toolchain))
        buf.extend(encode_string_field(3, self.command))
        buf.extend(encode_string_field(4, self.stderr))
        buf.extend(encode_string_field(5, self.stdout))
        buf.extend(encode_int32_field(6, self.exit_code))
        return bytes(buf)

    @classmethod
    def decode(cls, data: bytes):
        obj = cls()
        offset = 0
        while offset < len(data):
            tag, offset = decode_varint(data, offset)
            fnum = tag >> 3
            wtype = tag & 0x07
            if fnum == 1 and wtype == 2:
                obj.task_id, offset = decode_string_field(data, offset)
            elif fnum == 2 and wtype == 2:
                obj.toolchain, offset = decode_string_field(data, offset)
            elif fnum == 3 and wtype == 2:
                obj.command, offset = decode_string_field(data, offset)
            elif fnum == 4 and wtype == 2:
                obj.stderr, offset = decode_string_field(data, offset)
            elif fnum == 5 and wtype == 2:
                obj.stdout, offset = decode_string_field(data, offset)
            elif fnum == 6 and wtype == 0:
                obj.exit_code, offset = decode_varint(data, offset)
            else:
                offset = skip_field(data, offset, wtype)
        return obj

@dataclass
class FailureAnalysisResponse:
    error_category: str = ""
    root_cause: str = ""
    confidence: float = 0.0
    suggested_fixes: List[str] = field(default_factory=list)
    affected_files: List[str] = field(default_factory=list)

    def encode(self) -> bytes:
        buf = bytearray()
        buf.extend(encode_string_field(1, self.error_category))
        buf.extend(encode_string_field(2, self.root_cause))
        buf.extend(encode_double_field(3, self.confidence))
        buf.extend(encode_repeated_string(4, self.suggested_fixes))
        buf.extend(encode_repeated_string(5, self.affected_files))
        return bytes(buf)

    @classmethod
    def decode(cls, data: bytes):
        obj = cls()
        offset = 0
        while offset < len(data):
            tag, offset = decode_varint(data, offset)
            fnum = tag >> 3
            wtype = tag & 0x07
            if fnum == 1 and wtype == 2:
                obj.error_category, offset = decode_string_field(data, offset)
            elif fnum == 2 and wtype == 2:
                obj.root_cause, offset = decode_string_field(data, offset)
            elif fnum == 3 and wtype == 1:
                obj.confidence, offset = decode_double_field(data, offset)
            elif fnum == 4 and wtype == 2:
                s, offset = decode_string_field(data, offset)
                obj.suggested_fixes.append(s)
            elif fnum == 5 and wtype == 2:
                s, offset = decode_string_field(data, offset)
                obj.affected_files.append(s)
            else:
                offset = skip_field(data, offset, wtype)
        return obj

@dataclass
class WorkerRegistration:
    worker_id: str = ""
    address: str = ""
    cpu_cores: int = 0
    memory_bytes: int = 0
    supported_toolchains: List[str] = field(default_factory=list)
    tags: Dict[str, str] = field(default_factory=dict)

    def encode(self) -> bytes:
        buf = bytearray()
        buf.extend(encode_string_field(1, self.worker_id))
        buf.extend(encode_string_field(2, self.address))
        buf.extend(encode_int32_field(3, self.cpu_cores))
        buf.extend(encode_int64_field(4, self.memory_bytes))
        buf.extend(encode_repeated_string(5, self.supported_toolchains))
        if self.tags:
            buf.extend(encode_map_string_string(6, self.tags))
        return bytes(buf)

    @classmethod
    def decode(cls, data: bytes):
        obj = cls()
        offset = 0
        while offset < len(data):
            tag, offset = decode_varint(data, offset)
            fnum = tag >> 3
            wtype = tag & 0x07
            if fnum == 1 and wtype == 2:
                obj.worker_id, offset = decode_string_field(data, offset)
            elif fnum == 2 and wtype == 2:
                obj.address, offset = decode_string_field(data, offset)
            elif fnum == 3 and wtype == 0:
                obj.cpu_cores, offset = decode_varint(data, offset)
            elif fnum == 4 and wtype == 0:
                obj.memory_bytes, offset = decode_varint(data, offset)
            elif fnum == 5 and wtype == 2:
                s, offset = decode_string_field(data, offset)
                obj.supported_toolchains.append(s)
            elif fnum == 6 and wtype == 2:
                offset = decode_map_string_string_entry(data, offset, obj.tags)
            else:
                offset = skip_field(data, offset, wtype)
        return obj

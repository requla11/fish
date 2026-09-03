use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[inline]
pub fn encode_varint(mut val: u64, buf: &mut Vec<u8>) {
    while val >= 0x80 {
        buf.push((val as u8 & 0x7F) | 0x80);
        val >>= 7;
    }
    buf.push(val as u8);
}

#[inline]
pub fn decode_varint(buf: &[u8], offset: &mut usize) -> Result<u64, String> {
    let mut result = 0u64;
    let mut shift = 0;
    while *offset < buf.len() {
        let byte = buf[*offset];
        *offset += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err("varint overflow".to_string());
        }
    }
    Err("unexpected EOF while decoding varint".to_string())
}

#[inline]
pub fn encode_tag(field_num: u32, wire_type: u8, buf: &mut Vec<u8>) {
    let tag = ((field_num as u64) << 3) | (wire_type as u64);
    encode_varint(tag, buf);
}

pub fn skip_field(wire_type: u8, buf: &[u8], offset: &mut usize) -> Result<(), String> {
    match wire_type {
        0 => {
            decode_varint(buf, offset)?;
            Ok(())
        }
        1 => {
            if *offset + 8 > buf.len() {
                return Err("unexpected EOF for 64-bit field".to_string());
            }
            *offset += 8;
            Ok(())
        }
        2 => {
            let len = decode_varint(buf, offset)? as usize;
            if *offset + len > buf.len() {
                return Err("unexpected EOF for length-delimited field".to_string());
            }
            *offset += len;
            Ok(())
        }
        5 => {
            if *offset + 4 > buf.len() {
                return Err("unexpected EOF for 32-bit field".to_string());
            }
            *offset += 4;
            Ok(())
        }
        _ => Err(format!("unsupported wire type: {wire_type}")),
    }
}

pub fn encode_string_field(field_num: u32, val: &str, buf: &mut Vec<u8>) {
    if !val.is_empty() {
        encode_tag(field_num, 2, buf);
        encode_varint(val.len() as u64, buf);
        buf.extend_from_slice(val.as_bytes());
    }
}

pub fn decode_string_field(buf: &[u8], offset: &mut usize) -> Result<String, String> {
    let len = decode_varint(buf, offset)? as usize;
    if *offset + len > buf.len() {
        return Err("unexpected EOF reading string".to_string());
    }
    let s = std::str::from_utf8(&buf[*offset..*offset + len])
        .map_err(|e| e.to_string())?
        .to_string();
    *offset += len;
    Ok(s)
}

pub fn encode_int32_field(field_num: u32, val: i32, buf: &mut Vec<u8>) {
    if val != 0 {
        encode_tag(field_num, 0, buf);
        encode_varint(val as u64, buf);
    }
}

pub fn encode_int64_field(field_num: u32, val: i64, buf: &mut Vec<u8>) {
    if val != 0 {
        encode_tag(field_num, 0, buf);
        encode_varint(val as u64, buf);
    }
}

pub fn encode_bool_field(field_num: u32, val: bool, buf: &mut Vec<u8>) {
    if val {
        encode_tag(field_num, 0, buf);
        encode_varint(1, buf);
    }
}

pub fn encode_double_field(field_num: u32, val: f64, buf: &mut Vec<u8>) {
    if val != 0.0 {
        encode_tag(field_num, 1, buf);
        buf.extend_from_slice(&val.to_le_bytes());
    }
}

pub fn decode_double_field(buf: &[u8], offset: &mut usize) -> Result<f64, String> {
    if *offset + 8 > buf.len() {
        return Err("unexpected EOF reading double".to_string());
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&buf[*offset..*offset + 8]);
    *offset += 8;
    Ok(f64::from_le_bytes(bytes))
}

pub fn encode_repeated_string(field_num: u32, vals: &[String], buf: &mut Vec<u8>) {
    for val in vals {
        encode_string_field(field_num, val, buf);
    }
}

pub fn encode_map_string_string(field_num: u32, map: &HashMap<String, String>, buf: &mut Vec<u8>) {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    for k in keys {
        let v = &map[k];
        let mut entry_buf = Vec::new();
        encode_string_field(1, k, &mut entry_buf);
        encode_string_field(2, v, &mut entry_buf);
        encode_tag(field_num, 2, buf);
        encode_varint(entry_buf.len() as u64, buf);
        buf.extend_from_slice(&entry_buf);
    }
}

pub fn decode_map_string_string_entry(
    buf: &[u8],
    offset: &mut usize,
    map: &mut HashMap<String, String>,
) -> Result<(), String> {
    let len = decode_varint(buf, offset)? as usize;
    if *offset + len > buf.len() {
        return Err("unexpected EOF in map entry".to_string());
    }
    let end = *offset + len;
    let mut k = String::new();
    let mut v = String::new();
    while *offset < end {
        let tag = decode_varint(buf, offset)?;
        let fnum = (tag >> 3) as u32;
        let wtype = (tag & 0x07) as u8;
        match fnum {
            1 if wtype == 2 => k = decode_string_field(buf, offset)?,
            2 if wtype == 2 => v = decode_string_field(buf, offset)?,
            _ => skip_field(wtype, buf, offset)?,
        }
    }
    map.insert(k, v);
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BuildTask {
    pub id: String,
    pub package_name: String,
    pub toolchain: String,
    pub command: String,
    pub args: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub dependencies: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout_ms: i64,
}

impl BuildTask {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.id, &mut buf);
        encode_string_field(2, &self.package_name, &mut buf);
        encode_string_field(3, &self.toolchain, &mut buf);
        encode_string_field(4, &self.command, &mut buf);
        encode_repeated_string(5, &self.args, &mut buf);
        encode_repeated_string(6, &self.inputs, &mut buf);
        encode_repeated_string(7, &self.outputs, &mut buf);
        encode_repeated_string(8, &self.dependencies, &mut buf);
        encode_map_string_string(9, &self.env, &mut buf);
        encode_int64_field(10, self.timeout_ms, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut task = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => task.id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => task.package_name = decode_string_field(buf, &mut offset)?,
                3 if wtype == 2 => task.toolchain = decode_string_field(buf, &mut offset)?,
                4 if wtype == 2 => task.command = decode_string_field(buf, &mut offset)?,
                5 if wtype == 2 => task.args.push(decode_string_field(buf, &mut offset)?),
                6 if wtype == 2 => task.inputs.push(decode_string_field(buf, &mut offset)?),
                7 if wtype == 2 => task.outputs.push(decode_string_field(buf, &mut offset)?),
                8 if wtype == 2 => task
                    .dependencies
                    .push(decode_string_field(buf, &mut offset)?),
                9 if wtype == 2 => decode_map_string_string_entry(buf, &mut offset, &mut task.env)?,
                10 if wtype == 0 => task.timeout_ms = decode_varint(buf, &mut offset)? as i64,
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(task)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskResult {
    pub task_id: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i64,
    pub cached: bool,
    pub fingerprint: String,
    pub output_digests: HashMap<String, String>,
}

impl TaskResult {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.task_id, &mut buf);
        encode_int32_field(2, self.exit_code, &mut buf);
        encode_string_field(3, &self.stdout, &mut buf);
        encode_string_field(4, &self.stderr, &mut buf);
        encode_int64_field(5, self.duration_ms, &mut buf);
        encode_bool_field(6, self.cached, &mut buf);
        encode_string_field(7, &self.fingerprint, &mut buf);
        encode_map_string_string(8, &self.output_digests, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut res = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => res.task_id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 0 => res.exit_code = decode_varint(buf, &mut offset)? as i32,
                3 if wtype == 2 => res.stdout = decode_string_field(buf, &mut offset)?,
                4 if wtype == 2 => res.stderr = decode_string_field(buf, &mut offset)?,
                5 if wtype == 0 => res.duration_ms = decode_varint(buf, &mut offset)? as i64,
                6 if wtype == 0 => res.cached = decode_varint(buf, &mut offset)? != 0,
                7 if wtype == 2 => res.fingerprint = decode_string_field(buf, &mut offset)?,
                8 if wtype == 2 => {
                    decode_map_string_string_entry(buf, &mut offset, &mut res.output_digests)?
                }
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(res)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BuildGraph {
    pub root_package: String,
    pub tasks: Vec<BuildTask>,
    pub execution_order: Vec<String>,
}

impl BuildGraph {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.root_package, &mut buf);
        for task in &self.tasks {
            let task_bytes = task.encode();
            encode_tag(2, 2, &mut buf);
            encode_varint(task_bytes.len() as u64, &mut buf);
            buf.extend_from_slice(&task_bytes);
        }
        encode_repeated_string(3, &self.execution_order, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut graph = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => graph.root_package = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => {
                    let len = decode_varint(buf, &mut offset)? as usize;
                    if offset + len > buf.len() {
                        return Err("unexpected EOF in task message".to_string());
                    }
                    let task = BuildTask::decode(&buf[offset..offset + len])?;
                    offset += len;
                    graph.tasks.push(task);
                }
                3 if wtype == 2 => graph
                    .execution_order
                    .push(decode_string_field(buf, &mut offset)?),
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(graph)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiagnosticReport {
    pub run_id: String,
    pub status: String,
    pub total_duration_ms: i64,
    pub tasks_total: i32,
    pub tasks_succeeded: i32,
    pub tasks_failed: i32,
    pub tasks_cached: i32,
    pub failed_tasks: Vec<TaskResult>,
}

impl DiagnosticReport {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.run_id, &mut buf);
        encode_string_field(2, &self.status, &mut buf);
        encode_int64_field(3, self.total_duration_ms, &mut buf);
        encode_int32_field(4, self.tasks_total, &mut buf);
        encode_int32_field(5, self.tasks_succeeded, &mut buf);
        encode_int32_field(6, self.tasks_failed, &mut buf);
        encode_int32_field(7, self.tasks_cached, &mut buf);
        for ft in &self.failed_tasks {
            let ft_bytes = ft.encode();
            encode_tag(8, 2, &mut buf);
            encode_varint(ft_bytes.len() as u64, &mut buf);
            buf.extend_from_slice(&ft_bytes);
        }
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut rep = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => rep.run_id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => rep.status = decode_string_field(buf, &mut offset)?,
                3 if wtype == 0 => rep.total_duration_ms = decode_varint(buf, &mut offset)? as i64,
                4 if wtype == 0 => rep.tasks_total = decode_varint(buf, &mut offset)? as i32,
                5 if wtype == 0 => rep.tasks_succeeded = decode_varint(buf, &mut offset)? as i32,
                6 if wtype == 0 => rep.tasks_failed = decode_varint(buf, &mut offset)? as i32,
                7 if wtype == 0 => rep.tasks_cached = decode_varint(buf, &mut offset)? as i32,
                8 if wtype == 2 => {
                    let len = decode_varint(buf, &mut offset)? as usize;
                    if offset + len > buf.len() {
                        return Err("unexpected EOF in failed task".to_string());
                    }
                    let res = TaskResult::decode(&buf[offset..offset + len])?;
                    offset += len;
                    rep.failed_tasks.push(res);
                }
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(rep)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FailureAnalysisRequest {
    pub task_id: String,
    pub toolchain: String,
    pub command: String,
    pub stderr: String,
    pub stdout: String,
    pub exit_code: i32,
}

impl FailureAnalysisRequest {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.task_id, &mut buf);
        encode_string_field(2, &self.toolchain, &mut buf);
        encode_string_field(3, &self.command, &mut buf);
        encode_string_field(4, &self.stderr, &mut buf);
        encode_string_field(5, &self.stdout, &mut buf);
        encode_int32_field(6, self.exit_code, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut req = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => req.task_id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => req.toolchain = decode_string_field(buf, &mut offset)?,
                3 if wtype == 2 => req.command = decode_string_field(buf, &mut offset)?,
                4 if wtype == 2 => req.stderr = decode_string_field(buf, &mut offset)?,
                5 if wtype == 2 => req.stdout = decode_string_field(buf, &mut offset)?,
                6 if wtype == 0 => req.exit_code = decode_varint(buf, &mut offset)? as i32,
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(req)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FailureAnalysisResponse {
    pub error_category: String,
    pub root_cause: String,
    pub confidence: f64,
    pub suggested_fixes: Vec<String>,
    pub affected_files: Vec<String>,
}

impl FailureAnalysisResponse {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.error_category, &mut buf);
        encode_string_field(2, &self.root_cause, &mut buf);
        encode_double_field(3, self.confidence, &mut buf);
        encode_repeated_string(4, &self.suggested_fixes, &mut buf);
        encode_repeated_string(5, &self.affected_files, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut resp = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => resp.error_category = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => resp.root_cause = decode_string_field(buf, &mut offset)?,
                3 if wtype == 1 => resp.confidence = decode_double_field(buf, &mut offset)?,
                4 if wtype == 2 => resp
                    .suggested_fixes
                    .push(decode_string_field(buf, &mut offset)?),
                5 if wtype == 2 => resp
                    .affected_files
                    .push(decode_string_field(buf, &mut offset)?),
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(resp)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkerRegistration {
    pub worker_id: String,
    pub address: String,
    pub cpu_cores: i32,
    pub memory_bytes: i64,
    pub supported_toolchains: Vec<String>,
    pub tags: HashMap<String, String>,
}

impl WorkerRegistration {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.worker_id, &mut buf);
        encode_string_field(2, &self.address, &mut buf);
        encode_int32_field(3, self.cpu_cores, &mut buf);
        encode_int64_field(4, self.memory_bytes, &mut buf);
        encode_repeated_string(5, &self.supported_toolchains, &mut buf);
        encode_map_string_string(6, &self.tags, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut reg = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => reg.worker_id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => reg.address = decode_string_field(buf, &mut offset)?,
                3 if wtype == 0 => reg.cpu_cores = decode_varint(buf, &mut offset)? as i32,
                4 if wtype == 0 => reg.memory_bytes = decode_varint(buf, &mut offset)? as i64,
                5 if wtype == 2 => reg
                    .supported_toolchains
                    .push(decode_string_field(buf, &mut offset)?),
                6 if wtype == 2 => decode_map_string_string_entry(buf, &mut offset, &mut reg.tags)?,
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(reg)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkerHeartbeat {
    pub worker_id: String,
    pub cpu_load: f64,
    pub available_memory_bytes: i64,
    pub active_jobs: i32,
    pub timestamp: i64,
}

impl WorkerHeartbeat {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.worker_id, &mut buf);
        encode_double_field(2, self.cpu_load, &mut buf);
        encode_int64_field(3, self.available_memory_bytes, &mut buf);
        encode_int32_field(4, self.active_jobs, &mut buf);
        encode_int64_field(5, self.timestamp, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut hb = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => hb.worker_id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 1 => hb.cpu_load = decode_double_field(buf, &mut offset)?,
                3 if wtype == 0 => {
                    hb.available_memory_bytes = decode_varint(buf, &mut offset)? as i64
                }
                4 if wtype == 0 => hb.active_jobs = decode_varint(buf, &mut offset)? as i32,
                5 if wtype == 0 => hb.timestamp = decode_varint(buf, &mut offset)? as i64,
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(hb)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HeartbeatAck {
    pub accepted: bool,
    pub next_heartbeat_interval_ms: i64,
}

impl HeartbeatAck {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_bool_field(1, self.accepted, &mut buf);
        encode_int64_field(2, self.next_heartbeat_interval_ms, &mut buf);
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut ack = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 0 => ack.accepted = decode_varint(buf, &mut offset)? != 0,
                2 if wtype == 0 => {
                    ack.next_heartbeat_interval_ms = decode_varint(buf, &mut offset)? as i64
                }
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(ack)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskAssignment {
    pub job_id: String,
    pub task: Option<BuildTask>,
}

impl TaskAssignment {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.job_id, &mut buf);
        if let Some(ref t) = self.task {
            let tb = t.encode();
            encode_tag(2, 2, &mut buf);
            encode_varint(tb.len() as u64, &mut buf);
            buf.extend_from_slice(&tb);
        }
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut assign = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => assign.job_id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => {
                    let len = decode_varint(buf, &mut offset)? as usize;
                    if offset + len > buf.len() {
                        return Err("unexpected EOF in assigned task".to_string());
                    }
                    let t = BuildTask::decode(&buf[offset..offset + len])?;
                    offset += len;
                    assign.task = Some(t);
                }
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(assign)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskStatusUpdate {
    pub job_id: String,
    pub task_id: String,
    pub state: String,
    pub result: Option<TaskResult>,
}

impl TaskStatusUpdate {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        encode_string_field(1, &self.job_id, &mut buf);
        encode_string_field(2, &self.task_id, &mut buf);
        encode_string_field(3, &self.state, &mut buf);
        if let Some(ref r) = self.result {
            let rb = r.encode();
            encode_tag(4, 2, &mut buf);
            encode_varint(rb.len() as u64, &mut buf);
            buf.extend_from_slice(&rb);
        }
        buf
    }

    pub fn decode(buf: &[u8]) -> Result<Self, String> {
        let mut update = Self::default();
        let mut offset = 0;
        while offset < buf.len() {
            let tag = decode_varint(buf, &mut offset)?;
            let fnum = (tag >> 3) as u32;
            let wtype = (tag & 0x07) as u8;
            match fnum {
                1 if wtype == 2 => update.job_id = decode_string_field(buf, &mut offset)?,
                2 if wtype == 2 => update.task_id = decode_string_field(buf, &mut offset)?,
                3 if wtype == 2 => update.state = decode_string_field(buf, &mut offset)?,
                4 if wtype == 2 => {
                    let len = decode_varint(buf, &mut offset)? as usize;
                    if offset + len > buf.len() {
                        return Err("unexpected EOF in task result".to_string());
                    }
                    let r = TaskResult::decode(&buf[offset..offset + len])?;
                    offset += len;
                    update.result = Some(r);
                }
                _ => skip_field(wtype, buf, &mut offset)?,
            }
        }
        Ok(update)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip() {
        let values = [0u64, 1, 127, 128, 300, 16384, u32::MAX as u64, u64::MAX];
        for v in values {
            let mut buf = Vec::new();
            encode_varint(v, &mut buf);
            let mut offset = 0;
            let decoded = decode_varint(&buf, &mut offset).unwrap();
            assert_eq!(v, decoded);
            assert_eq!(offset, buf.len());
        }
    }

    #[test]
    fn test_build_task_roundtrip() {
        let mut env = HashMap::new();
        env.insert("RUST_BACKTRACE".to_string(), "1".to_string());
        env.insert("CARGO_TERM_COLOR".to_string(), "always".to_string());

        let task = BuildTask {
            id: "task-rust-01".to_string(),
            package_name: "fish-cli".to_string(),
            toolchain: "rust".to_string(),
            command: "cargo check".to_string(),
            args: vec!["--workspace".to_string(), "--all-targets".to_string()],
            inputs: vec!["src/**/*.rs".to_string(), "Cargo.toml".to_string()],
            outputs: vec!["target/debug/fish.exe".to_string()],
            dependencies: vec!["task-core-01".to_string()],
            env,
            timeout_ms: 60000,
        };

        let encoded = task.encode();
        assert!(!encoded.is_empty());
        let decoded = BuildTask::decode(&encoded).unwrap();
        assert_eq!(task, decoded);
    }

    #[test]
    fn test_task_result_roundtrip() {
        let mut digests = HashMap::new();
        digests.insert(
            "target/bin".to_string(),
            "blake3:abcdef0123456789".to_string(),
        );

        let res = TaskResult {
            task_id: "task-01".to_string(),
            exit_code: 0,
            stdout: "Build successful\n".to_string(),
            stderr: String::new(),
            duration_ms: 1250,
            cached: true,
            fingerprint: "blake3:1122334455667788".to_string(),
            output_digests: digests,
        };

        let encoded = res.encode();
        let decoded = TaskResult::decode(&encoded).unwrap();
        assert_eq!(res, decoded);
    }

    #[test]
    fn test_build_graph_roundtrip() {
        let task = BuildTask {
            id: "t1".to_string(),
            package_name: "pkg1".to_string(),
            toolchain: "go".to_string(),
            command: "go build".to_string(),
            args: vec!["./...".to_string()],
            inputs: vec!["*.go".to_string()],
            outputs: vec!["bin/pkg1".to_string()],
            dependencies: Vec::new(),
            env: HashMap::new(),
            timeout_ms: 30000,
        };

        let graph = BuildGraph {
            root_package: "fish-monorepo".to_string(),
            tasks: vec![task],
            execution_order: vec!["t1".to_string()],
        };

        let encoded = graph.encode();
        let decoded = BuildGraph::decode(&encoded).unwrap();
        assert_eq!(graph, decoded);
    }

    #[test]
    fn test_diagnostic_report_roundtrip() {
        let res = TaskResult {
            task_id: "t-fail".to_string(),
            exit_code: 101,
            stdout: String::new(),
            stderr: "error[E0425]: cannot find value `foo`".to_string(),
            duration_ms: 340,
            cached: false,
            fingerprint: "abc".to_string(),
            output_digests: HashMap::new(),
        };

        let rep = DiagnosticReport {
            run_id: "run-99".to_string(),
            status: "failed".to_string(),
            total_duration_ms: 450,
            tasks_total: 10,
            tasks_succeeded: 9,
            tasks_failed: 1,
            tasks_cached: 5,
            failed_tasks: vec![res],
        };

        let encoded = rep.encode();
        let decoded = DiagnosticReport::decode(&encoded).unwrap();
        assert_eq!(rep, decoded);
    }

    #[test]
    fn test_failure_analysis_roundtrip() {
        let req = FailureAnalysisRequest {
            task_id: "task-py".to_string(),
            toolchain: "python".to_string(),
            command: "pytest".to_string(),
            stderr: "ModuleNotFoundError: No module named 'fish'".to_string(),
            stdout: "collected 0 items".to_string(),
            exit_code: 1,
        };

        let encoded = req.encode();
        let decoded = FailureAnalysisRequest::decode(&encoded).unwrap();
        assert_eq!(req, decoded);

        let resp = FailureAnalysisResponse {
            error_category: "missing_dependency".to_string(),
            root_cause: "python package 'fish' is not installed".to_string(),
            confidence: 0.95,
            suggested_fixes: vec!["pip install -e .".to_string()],
            affected_files: vec!["setup.py".to_string(), "pyproject.toml".to_string()],
        };

        let resp_encoded = resp.encode();
        let resp_decoded = FailureAnalysisResponse::decode(&resp_encoded).unwrap();
        assert_eq!(resp.error_category, resp_decoded.error_category);
        assert_eq!(resp.root_cause, resp_decoded.root_cause);
        assert!((resp.confidence - resp_decoded.confidence).abs() < 1e-6);
        assert_eq!(resp.suggested_fixes, resp_decoded.suggested_fixes);
        assert_eq!(resp.affected_files, resp_decoded.affected_files);
    }

    #[test]
    fn test_coordinator_messages_roundtrip() {
        let mut tags = HashMap::new();
        tags.insert("arch".to_string(), "x86_64".to_string());
        tags.insert("os".to_string(), "linux".to_string());

        let reg = WorkerRegistration {
            worker_id: "worker-01".to_string(),
            address: "192.168.1.100:9090".to_string(),
            cpu_cores: 16,
            memory_bytes: 32 * 1024 * 1024 * 1024,
            supported_toolchains: vec!["rust".to_string(), "go".to_string(), "docker".to_string()],
            tags,
        };

        let reg_enc = reg.encode();
        let reg_dec = WorkerRegistration::decode(&reg_enc).unwrap();
        assert_eq!(reg, reg_dec);

        let hb = WorkerHeartbeat {
            worker_id: "worker-01".to_string(),
            cpu_load: 0.42,
            available_memory_bytes: 16 * 1024 * 1024 * 1024,
            active_jobs: 3,
            timestamp: 1725360000,
        };

        let hb_enc = hb.encode();
        let hb_dec = WorkerHeartbeat::decode(&hb_enc).unwrap();
        assert_eq!(hb.worker_id, hb_dec.worker_id);
        assert!((hb.cpu_load - hb_dec.cpu_load).abs() < 1e-6);
        assert_eq!(hb.available_memory_bytes, hb_dec.available_memory_bytes);
        assert_eq!(hb.active_jobs, hb_dec.active_jobs);
        assert_eq!(hb.timestamp, hb_dec.timestamp);

        let ack = HeartbeatAck {
            accepted: true,
            next_heartbeat_interval_ms: 5000,
        };
        let ack_enc = ack.encode();
        let ack_dec = HeartbeatAck::decode(&ack_enc).unwrap();
        assert_eq!(ack, ack_dec);

        let task = BuildTask {
            id: "t-dispatch".to_string(),
            package_name: "app".to_string(),
            toolchain: "rust".to_string(),
            command: "cargo test".to_string(),
            args: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            dependencies: Vec::new(),
            env: HashMap::new(),
            timeout_ms: 10000,
        };

        let assign = TaskAssignment {
            job_id: "job-888".to_string(),
            task: Some(task),
        };
        let assign_enc = assign.encode();
        let assign_dec = TaskAssignment::decode(&assign_enc).unwrap();
        assert_eq!(assign, assign_dec);

        let update = TaskStatusUpdate {
            job_id: "job-888".to_string(),
            task_id: "t-dispatch".to_string(),
            state: "completed".to_string(),
            result: Some(TaskResult {
                task_id: "t-dispatch".to_string(),
                exit_code: 0,
                stdout: "ok".to_string(),
                stderr: String::new(),
                duration_ms: 120,
                cached: false,
                fingerprint: "hash".to_string(),
                output_digests: HashMap::new(),
            }),
        };
        let update_enc = update.encode();
        let update_dec = TaskStatusUpdate::decode(&update_enc).unwrap();
        assert_eq!(update, update_dec);
    }
}

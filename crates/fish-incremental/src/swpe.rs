use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WaveletEnergy {
    TrivialWhitespace,
    CommentOnly,
    InternalStatement,
    FunctionBoundary,
    GlobalInterface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeystrokeWavelet {
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub char_delta: isize,
    pub timestamp_ms: u64,
    pub new_content: String,
}

impl KeystrokeWavelet {
    pub fn assess_energy(&self, prev_content: Option<&str>) -> WaveletEnergy {
        let trimmed = self.new_content.trim();
        if trimmed.is_empty() {
            return WaveletEnergy::TrivialWhitespace;
        }
        if prev_content.is_some_and(|prev| prev.trim() == trimmed) {
            return WaveletEnergy::TrivialWhitespace;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('#') {
            return WaveletEnergy::CommentOnly;
        }
        if trimmed.contains("pub fn")
            || trimmed.contains("export function")
            || trimmed.contains("func ")
            || trimmed.contains("class ")
            || trimmed.contains("interface ")
        {
            return WaveletEnergy::GlobalInterface;
        }
        if trimmed.contains("fn ")
            || trimmed.contains("def ")
            || trimmed.contains("let ")
            || trimmed.contains("const ")
        {
            return WaveletEnergy::FunctionBoundary;
        }
        WaveletEnergy::InternalStatement
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreWarmedArtifact {
    pub file_path: PathBuf,
    pub wavelet_id: u64,
    pub source_hash: [u8; 32],
    pub inferred_symbols: Vec<String>,
    pub precompiled_ir: Vec<u8>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SpeculativeRingBuffer {
    capacity: usize,
    buffer: VecDeque<PreWarmedArtifact>,
}

impl SpeculativeRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            buffer: VecDeque::with_capacity(capacity.max(1)),
        }
    }

    pub fn push(&mut self, artifact: PreWarmedArtifact) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(artifact);
    }

    pub fn claim(
        &mut self,
        file_path: &Path,
        current_hash: &[u8; 32],
    ) -> Option<PreWarmedArtifact> {
        if let Some(idx) = self
            .buffer
            .iter()
            .rposition(|a| a.file_path == file_path && a.source_hash == *current_hash)
        {
            return self.buffer.remove(idx);
        }
        None
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct WaveletSchedulerEngine {
    ring_buffer: SpeculativeRingBuffer,
    last_snapshots: HashMap<PathBuf, String>,
    wavelet_counter: u64,
}

impl Default for WaveletSchedulerEngine {
    fn default() -> Self {
        Self::new(32)
    }
}

impl WaveletSchedulerEngine {
    pub fn new(ring_capacity: usize) -> Self {
        Self {
            ring_buffer: SpeculativeRingBuffer::new(ring_capacity),
            last_snapshots: HashMap::new(),
            wavelet_counter: 0,
        }
    }

    pub fn on_keystroke_wavelet(
        &mut self,
        file_path: &Path,
        new_content: &str,
        timestamp_ms: u64,
    ) -> (WaveletEnergy, Option<PreWarmedArtifact>) {
        self.wavelet_counter += 1;
        let prev = self.last_snapshots.get(file_path).map(String::as_str);

        let wavelet = KeystrokeWavelet {
            file_path: file_path.to_path_buf(),
            line_start: 1,
            line_end: new_content.lines().count(),
            char_delta: new_content.len() as isize - prev.map(|p| p.len() as isize).unwrap_or(0),
            timestamp_ms,
            new_content: new_content.to_string(),
        };

        let energy = wavelet.assess_energy(prev);
        self.last_snapshots
            .insert(file_path.to_path_buf(), new_content.to_string());

        if energy >= WaveletEnergy::InternalStatement {
            let mut hasher = blake3::Hasher::new();
            hasher.update(file_path.to_string_lossy().as_bytes());
            hasher.update(new_content.as_bytes());
            let source_hash = *hasher.finalize().as_bytes();

            let mut symbols = Vec::new();
            for line in new_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("export function ")
                {
                    symbols.push(trimmed.to_string());
                }
            }

            let dummy_ir =
                format!("IR_BLOB_V{}_{}", self.wavelet_counter, symbols.len()).into_bytes();

            let artifact = PreWarmedArtifact {
                file_path: file_path.to_path_buf(),
                wavelet_id: self.wavelet_counter,
                source_hash,
                inferred_symbols: symbols,
                precompiled_ir: dummy_ir,
                timestamp_ms,
            };

            self.ring_buffer.push(artifact.clone());
            return (energy, Some(artifact));
        }

        (energy, None)
    }

    pub fn trigger_instant_build(
        &mut self,
        file_path: &Path,
        content: &str,
    ) -> Option<PreWarmedArtifact> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        hasher.update(content.as_bytes());
        let current_hash = *hasher.finalize().as_bytes();
        self.ring_buffer.claim(file_path, &current_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wavelet_energy_classification() {
        let path = Path::new("src/main.rs");
        let w_space = KeystrokeWavelet {
            file_path: path.to_path_buf(),
            line_start: 1,
            line_end: 1,
            char_delta: 2,
            timestamp_ms: 100,
            new_content: "   \n".to_string(),
        };
        assert_eq!(
            w_space.assess_energy(None),
            WaveletEnergy::TrivialWhitespace
        );

        let w_comment = KeystrokeWavelet {
            file_path: path.to_path_buf(),
            line_start: 1,
            line_end: 1,
            char_delta: 10,
            timestamp_ms: 101,
            new_content: "// just a note\n".to_string(),
        };
        assert_eq!(w_comment.assess_energy(None), WaveletEnergy::CommentOnly);

        let w_func = KeystrokeWavelet {
            file_path: path.to_path_buf(),
            line_start: 1,
            line_end: 2,
            char_delta: 25,
            timestamp_ms: 102,
            new_content: "pub fn compute_sum() -> i32 { 0 }\n".to_string(),
        };
        assert_eq!(w_func.assess_energy(None), WaveletEnergy::GlobalInterface);
    }

    #[test]
    fn test_speculative_pre_warm_and_instant_claim() {
        let mut engine = WaveletSchedulerEngine::new(8);
        let path = Path::new("src/engine.rs");
        let code = "pub fn execute() -> bool { true }\n";

        let (energy, prewarmed) = engine.on_keystroke_wavelet(path, code, 1000);
        assert_eq!(energy, WaveletEnergy::GlobalInterface);
        assert!(prewarmed.is_some());

        let claimed = engine.trigger_instant_build(path, code);
        assert!(claimed.is_some());
        assert_eq!(claimed.unwrap().inferred_symbols.len(), 1);

        let claimed_again = engine.trigger_instant_build(path, code);
        assert!(claimed_again.is_none());
    }
}

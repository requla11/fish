use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Static description of a WASM plugin's sandbox policy. The runtime that
/// would enforce these limits (instantiation, fuel metering, host-call
/// allow-listing) is not embedded in fish yet; the config is accepted and
/// validated so manifests can be authored today and executed once the
/// WebAssembly Plugin Engine milestone lands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginConfig {
    pub name: String,
    pub wasm_bytes_len: usize,
    pub allowed_hosts: Vec<String>,
    pub memory_limit_pages: u32,
    pub environment_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
}

pub struct WasmPluginSandbox {
    config: WasmPluginConfig,
    active: bool,
}

impl WasmPluginSandbox {
    pub fn new(config: WasmPluginConfig) -> Self {
        Self {
            config,
            active: true,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn config(&self) -> &WasmPluginConfig {
        &self.config
    }

    /// Validate the canonical WASM binary header: 4-byte magic `\0asm`
    /// followed by a little-endian version field.
    pub fn validate_wasm_header(bytes: &[u8]) -> Result<u32, String> {
        if bytes.len() < 8 {
            return Err("WASM binary is smaller than header length (8 bytes)".to_string());
        }

        let magic = &bytes[0..4];
        if magic != [0x00, 0x61, 0x73, 0x6D] {
            return Err("Invalid WASM magic bytes (expected \0asm)".to_string());
        }

        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != 1 {
            return Err(format!("Unsupported WASM binary version: {version}"));
        }

        Ok(version)
    }

    /// Invoke an exported function inside the sandbox.
    ///
    /// Fish does not embed a WASM interpreter yet, so every call fails loudly
    /// instead of fabricating a result. This keeps plugin pipelines honest
    /// until the WebAssembly Plugin Engine milestone ships a real runtime
    /// (wasmtime/wasmi) honoring [`WasmPluginConfig`].
    pub fn execute_function(&self, fn_name: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
        if !self.active {
            return Err("Sandbox terminated".to_string());
        }
        let _ = (fn_name, payload);
        Err(
            "WASM execution is not available: fish does not embed a WASM runtime yet. \
             The `fish fix`/plugin tooling will run modules once the WebAssembly \
             Plugin Engine milestone lands."
                .to_string(),
        )
    }

    pub fn terminate(&mut self) {
        self.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_header_validation() {
        let valid_wasm = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert_eq!(WasmPluginSandbox::validate_wasm_header(&valid_wasm), Ok(1));

        let invalid_magic = [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        assert!(WasmPluginSandbox::validate_wasm_header(&invalid_magic).is_err());

        let bad_version = [0x00, 0x61, 0x73, 0x6D, 0x63, 0x00, 0x00, 0x00];
        assert!(WasmPluginSandbox::validate_wasm_header(&bad_version).is_err());
    }

    #[test]
    fn test_execute_function_fails_loudly_without_runtime() {
        let config = WasmPluginConfig {
            name: "test-plugin".to_string(),
            wasm_bytes_len: 1024,
            allowed_hosts: vec!["api.fish.build".to_string()],
            memory_limit_pages: 16,
            environment_vars: HashMap::new(),
        };

        let mut sandbox = WasmPluginSandbox::new(config);
        assert!(sandbox.is_active());

        let res = sandbox.execute_function("hook_pre_build", b"payload");
        let err = res.expect_err("execution must fail while no runtime is embedded");
        assert!(err.contains("not available"), "got: {err}");

        sandbox.terminate();
        assert!(!sandbox.is_active());
        let res = sandbox.execute_function("hook_pre_build", b"payload");
        assert_eq!(res.unwrap_err(), "Sandbox terminated");
    }
}

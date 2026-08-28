use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VLinkJumpEntry {
    pub symbol_name: String,
    pub virtual_address: u64,
    pub code_offset: usize,
    pub code_length: usize,
    pub symbol_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualBinaryDispatchTable {
    pub binary_id: String,
    pub base_executable: PathBuf,
    pub jump_entries: HashMap<String, VLinkJumpEntry>,
    pub overlay_arena: Vec<u8>,
}

impl VirtualBinaryDispatchTable {
    pub fn new(binary_id: &str, base_executable: &Path) -> Self {
        Self {
            binary_id: binary_id.to_string(),
            base_executable: base_executable.to_path_buf(),
            jump_entries: HashMap::new(),
            overlay_arena: Vec::new(),
        }
    }

    pub fn register_symbol(&mut self, name: &str, virtual_address: u64, initial_code: &[u8]) {
        let code_offset = self.overlay_arena.len();
        self.overlay_arena.extend_from_slice(initial_code);
        let mut hasher = blake3::Hasher::new();
        hasher.update(initial_code);
        let symbol_hash = *hasher.finalize().as_bytes();

        self.jump_entries.insert(
            name.to_string(),
            VLinkJumpEntry {
                symbol_name: name.to_string(),
                virtual_address,
                code_offset,
                code_length: initial_code.len(),
                symbol_hash,
            },
        );
    }
}

pub struct VLinkSpliceEngine;

impl VLinkSpliceEngine {
    pub fn splice_symbol(
        table: &mut VirtualBinaryDispatchTable,
        symbol_name: &str,
        new_code: &[u8],
    ) -> Result<u64, String> {
        let entry = table
            .jump_entries
            .get_mut(symbol_name)
            .ok_or_else(|| format!("symbol `{symbol_name}` not registered in VBDT"))?;

        let mut hasher = blake3::Hasher::new();
        hasher.update(new_code);
        let new_hash = *hasher.finalize().as_bytes();

        if entry.symbol_hash == new_hash {
            return Ok(entry.virtual_address);
        }

        let new_offset = table.overlay_arena.len();
        table.overlay_arena.extend_from_slice(new_code);

        let new_virtual_address = 0x400000 + new_offset as u64;

        entry.code_offset = new_offset;
        entry.code_length = new_code.len();
        entry.symbol_hash = new_hash;
        entry.virtual_address = new_virtual_address;

        Ok(new_virtual_address)
    }

    pub fn emit_runtime_overlay(table: &VirtualBinaryDispatchTable) -> Vec<u8> {
        let mut image = Vec::with_capacity(table.overlay_arena.len() + 64);
        image.extend_from_slice(b"VLINK_DISPATCH_HEADER_V1");
        let count = table.jump_entries.len() as u32;
        image.extend_from_slice(&count.to_le_bytes());
        for entry in table.jump_entries.values() {
            let name_bytes = entry.symbol_name.as_bytes();
            image.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            image.extend_from_slice(name_bytes);
            image.extend_from_slice(&entry.virtual_address.to_le_bytes());
            image.extend_from_slice(&(entry.code_offset as u32).to_le_bytes());
            image.extend_from_slice(&(entry.code_length as u32).to_le_bytes());
        }
        image.extend_from_slice(&table.overlay_arena);
        image
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlink_symbol_registration_and_splice() {
        let mut table = VirtualBinaryDispatchTable::new("my_app", Path::new("target/my_app.exe"));
        table.register_symbol("calc", 0x1000, b"\x90\x90\xc3");

        assert_eq!(table.jump_entries.len(), 1);
        let initial_addr = table.jump_entries["calc"].virtual_address;
        assert_eq!(initial_addr, 0x1000);

        let new_addr =
            VLinkSpliceEngine::splice_symbol(&mut table, "calc", b"\x48\x31\xc0\xc3").unwrap();
        assert_ne!(new_addr, initial_addr);
        assert_eq!(table.jump_entries["calc"].virtual_address, new_addr);
        assert_eq!(table.jump_entries["calc"].code_length, 4);
    }

    #[test]
    fn test_vlink_unchanged_symbol_no_op() {
        let mut table = VirtualBinaryDispatchTable::new("my_app", Path::new("target/my_app.exe"));
        table.register_symbol("main_entry", 0x2000, b"\xcc\xc3");

        let addr1 =
            VLinkSpliceEngine::splice_symbol(&mut table, "main_entry", b"\xcc\xc3").unwrap();
        assert_eq!(addr1, 0x2000);
        assert_eq!(table.overlay_arena.len(), 2);
    }

    #[test]
    fn test_emit_runtime_overlay_header() {
        let mut table = VirtualBinaryDispatchTable::new("app", Path::new("app.bin"));
        table.register_symbol("foo", 0x1000, b"\x90");
        let overlay = VLinkSpliceEngine::emit_runtime_overlay(&table);
        assert!(overlay.starts_with(b"VLINK_DISPATCH_HEADER_V1"));
    }
}

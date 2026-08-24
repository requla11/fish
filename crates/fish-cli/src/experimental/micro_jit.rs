#![allow(dead_code)]

use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitectureTarget {
    X86_64,
    AArch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitRegister {
    Rax,
    Rcx,
    Rdx,
}

#[derive(Debug, Clone)]
pub enum JitOpcode {
    /// `mov reg, imm32` — encoded with the B8+r opcode family.
    MovImm(JitRegister, i64),
    /// Two-register ALU op. Only the `rax <- rcx` encoding below is
    /// implemented; anything else is refused rather than mis-assembled.
    Add(JitRegister, JitRegister),
    Sub(JitRegister, JitRegister),
    Xor(JitRegister, JitRegister),
    Ret,
}

/// Result of assembling an instruction sequence.
///
/// `memory_address` is `None` because nothing here maps executable memory;
/// reporting a synthetic address would imply bytes exist somewhere they do
/// not. No execution timing is provided for the same reason.
#[derive(Debug, Clone)]
pub struct CompiledJitFunction {
    pub function_name: String,
    pub machine_opcodes: Vec<u8>,
    pub memory_address: Option<usize>,
    pub disassembly: Vec<String>,
}

pub struct MicroJitEngine {
    target: ArchitectureTarget,
}

impl MicroJitEngine {
    pub fn new(target: ArchitectureTarget) -> Self {
        Self { target }
    }

    pub fn compile_expression_to_machine_code(
        &self,
        name: &str,
        _constant_value: i32,
    ) -> io::Result<CompiledJitFunction> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("in-process micro-JIT is not implemented (cannot compile `{name}`)"),
        ))
    }

    fn unsupported(detail: &str) -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("micro-JIT assembler supports only its documented subset: {detail}"),
        )
    }

    /// Assemble a tiny, fully-specified x86-64 instruction subset.
    ///
    /// Every emitted byte corresponds exactly to the requested operands and
    /// every disassembly line matches the emitted bytes. Combinations outside
    /// the implemented encodings return `Unsupported` instead of producing
    /// plausible-looking garbage.
    pub fn assemble_function(
        &self,
        name: &str,
        operations: &[JitOpcode],
    ) -> io::Result<CompiledJitFunction> {
        match self.target {
            ArchitectureTarget::X86_64 => self.assemble_x86_64(name, operations),
            ArchitectureTarget::AArch64 => {
                Err(Self::unsupported("AArch64 encoding is not implemented"))
            }
        }
    }

    fn assemble_x86_64(
        &self,
        name: &str,
        operations: &[JitOpcode],
    ) -> io::Result<CompiledJitFunction> {
        let mut bytes = Vec::new();
        let mut disassembly = Vec::new();

        let mov_opcode = |reg: &JitRegister| -> Option<u8> {
            match reg {
                JitRegister::Rax => Some(0xB8),
                JitRegister::Rcx => Some(0xB9),
                JitRegister::Rdx => Some(0xBA),
            }
        };

        for op in operations {
            match op {
                JitOpcode::MovImm(reg, val) => {
                    let opcode = mov_opcode(reg)
                        .ok_or_else(|| Self::unsupported("MovImm is limited to rax/rcx/rdx"))?;
                    if !(-2_147_483_648..=2_147_483_647).contains(val) {
                        return Err(Self::unsupported("MovImm immediate exceeds i32"));
                    }
                    bytes.push(opcode);
                    bytes.extend_from_slice(&(*val as i32).to_le_bytes());
                    disassembly.push(format!("mov {:?}, {val}", reg));
                }
                JitOpcode::Add(dst, src) => {
                    if (*dst, *src) != (JitRegister::Rax, JitRegister::Rcx) {
                        return Err(Self::unsupported("add is limited to `add rax, rcx`"));
                    }
                    bytes.extend_from_slice(&[0x48, 0x01, 0xC8]);
                    disassembly.push("add rax, rcx".to_string());
                }
                JitOpcode::Sub(dst, src) => {
                    if (*dst, *src) != (JitRegister::Rax, JitRegister::Rcx) {
                        return Err(Self::unsupported("sub is limited to `sub rax, rcx`"));
                    }
                    bytes.extend_from_slice(&[0x48, 0x29, 0xC8]);
                    disassembly.push("sub rax, rcx".to_string());
                }
                JitOpcode::Xor(dst, src) => {
                    if (*dst, *src) != (JitRegister::Rax, JitRegister::Rcx) {
                        return Err(Self::unsupported("xor is limited to `xor rax, rcx`"));
                    }
                    bytes.extend_from_slice(&[0x48, 0x31, 0xC8]);
                    disassembly.push("xor rax, rcx".to_string());
                }
                JitOpcode::Ret => {
                    bytes.push(0xC3);
                    disassembly.push("ret".to_string());
                }
            }
        }

        Ok(CompiledJitFunction {
            function_name: name.to_string(),
            machine_opcodes: bytes,
            memory_address: None,
            disassembly,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assembles_supported_subset_truthfully() {
        let jit = MicroJitEngine::new(ArchitectureTarget::X86_64);
        let ops = vec![
            JitOpcode::MovImm(JitRegister::Rax, 100),
            JitOpcode::Add(JitRegister::Rax, JitRegister::Rcx),
            JitOpcode::Ret,
        ];

        let compiled = jit.assemble_function("compute_fast", &ops).unwrap();
        assert_eq!(compiled.function_name, "compute_fast");
        assert_eq!(
            compiled.disassembly,
            vec!["mov Rax, 100", "add rax, rcx", "ret"]
        );
        assert_eq!(
            compiled.machine_opcodes,
            vec![0xB8, 100, 0, 0, 0, 0x48, 0x01, 0xC8, 0xC3]
        );
        assert_eq!(compiled.memory_address, None, "nothing is mapped");
    }

    #[test]
    fn test_unsupported_operand_combinations_are_refused() {
        let jit = MicroJitEngine::new(ArchitectureTarget::X86_64);

        let bad_add = jit
            .assemble_function("bad", &[JitOpcode::Add(JitRegister::Rcx, JitRegister::Rdx)])
            .unwrap_err();
        assert_eq!(bad_add.kind(), io::ErrorKind::Unsupported);

        let big_imm = jit
            .assemble_function("big", &[JitOpcode::MovImm(JitRegister::Rax, 5_000_000_000)])
            .unwrap_err();
        assert_eq!(big_imm.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn test_aarch64_refused_until_implemented() {
        let jit = MicroJitEngine::new(ArchitectureTarget::AArch64);
        let err = jit.assemble_function("arm", &[JitOpcode::Ret]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }
}

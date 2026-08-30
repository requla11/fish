use banana::ledger::{InTotoStatement, LedgerWitness, MerkleTree};
use std::path::Path;

pub struct FishSlsaWitness {
    witness: LedgerWitness,
}

impl FishSlsaWitness {
    pub fn new() -> Self {
        Self {
            witness: LedgerWitness::new(),
        }
    }

    pub fn record_build_output(
        &mut self,
        artifact_name: impl Into<String>,
        artifact_hash: impl Into<String>,
        builder_id: impl Into<String>,
    ) -> (u64, InTotoStatement) {
        let seq = self
            .witness
            .append_record(artifact_name, artifact_hash, builder_id);
        let statement = self.witness.records()[seq as usize].to_slsa_v1_statement();
        (seq, statement)
    }

    pub fn build_and_sign_tree(&self) -> (MerkleTree, String) {
        let tree = self.witness.build_tree();
        let (sig, _) = self.witness.sign_root(&tree);
        (tree, sig)
    }

    pub fn persist_ledger(&self, path: &Path) -> Result<(), anyhow::Error> {
        self.witness.persist_to_disk(path)
    }
}

impl Default for FishSlsaWitness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fish_slsa_witness_integration() {
        let mut slsa = FishSlsaWitness::new();
        let (seq, stmt) = slsa.record_build_output("fish-cli", "blake3:feedbeef", "ci-builder");
        assert_eq!(seq, 0);
        assert_eq!(stmt.statement_type, "https://in-toto.io/Statement/v1");

        let (tree, sig) = slsa.build_and_sign_tree();
        assert!(!tree.root_hash().is_empty());
        assert!(!sig.is_empty());

        let temp = tempdir().unwrap();
        let log = temp.path().join("audit.jsonl");
        slsa.persist_ledger(&log).unwrap();
        assert!(log.exists());
    }
}

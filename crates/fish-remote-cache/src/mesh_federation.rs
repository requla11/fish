use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

use crate::banana_mesh::BananaMeshCache;
use fish_security::slsa::{
    SignedStatement, verify_slsa_level3_compliance, verify_statement_signature,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPayload {
    pub artifact_data: Vec<u8>,
    pub attestation: SignedStatement,
}

#[derive(Debug)]
pub enum FederationError {
    Network(String),
    Serialization(String),
    MissingAttestation,
    SlsaComplianceError(String),
    SignatureVerificationFailed(String),
    HashMismatch,
}

pub struct GlobalMeshFederation {
    mesh: BananaMeshCache,
    trusted_public_keys: HashSet<String>,
}

impl GlobalMeshFederation {
    pub fn new(mesh: BananaMeshCache) -> Self {
        Self {
            mesh,
            trusted_public_keys: HashSet::new(),
        }
    }

    pub fn add_trusted_key(&mut self, public_key_b64: &str) {
        self.trusted_public_keys.insert(public_key_b64.to_string());
    }

    pub fn publish_artifact(
        &self,
        blake3_hash: &str,
        data: Vec<u8>,
        attestation: SignedStatement,
    ) -> Result<(), FederationError> {
        let payload = FederationPayload {
            artifact_data: data,
            attestation,
        };

        let encoded = serde_json::to_vec(&payload)
            .map_err(|e| FederationError::Serialization(e.to_string()))?;

        self.mesh.store_artifact(blake3_hash, encoded);
        Ok(())
    }

    pub fn fetch_and_verify(
        &self,
        expected_blake3_hash: &str,
    ) -> Result<Option<Vec<u8>>, FederationError> {
        // Find peers holding the artifact via Kademlia DHT
        let peers = self.mesh.find_peer_nodes(expected_blake3_hash);
        if peers.is_empty() {
            return Ok(None);
        }

        // We simulate streaming from peers. The banana mesh handles the underlying byte retrieval.
        let raw_data = match self.mesh.get_local_artifact(expected_blake3_hash) {
            Some(data) => data,
            None => {
                // In a real network, we'd establish a P2P connection to the peer and stream.
                // For this moonshot prototype, we assume the banana mesh retrieves it.
                return Err(FederationError::Network(
                    "Failed to retrieve chunk from peers".to_string(),
                ));
            }
        };

        let payload: FederationPayload = serde_json::from_slice(&raw_data)
            .map_err(|e| FederationError::Serialization(e.to_string()))?;

        // 1. Verify cryptographic signature of the SLSA attestation
        let mut signature_valid = false;
        for trusted_key in &self.trusted_public_keys {
            if verify_statement_signature(&payload.attestation, trusted_key).is_ok() {
                signature_valid = true;
                break;
            }
        }

        if !signature_valid {
            return Err(FederationError::SignatureVerificationFailed(
                "No trusted key matched the attestation signature".to_string(),
            ));
        }

        // 2. Verify SLSA Level 3 Compliance (Hermetic, exact params, valid structure)
        verify_slsa_level3_compliance(&payload.attestation.statement)
            .map_err(|e| FederationError::SlsaComplianceError(e))?;

        // 3. Verify the subject in the attestation matches the requested hash
        let statement = &payload.attestation.statement;
        let mut subject_hash_matches = false;
        for subject in &statement.subject {
            if let Some(hash) = subject.digest.get("blake3") {
                if hash == expected_blake3_hash {
                    subject_hash_matches = true;
                    break;
                }
            }
        }

        if !subject_hash_matches {
            return Err(FederationError::SlsaComplianceError(format!(
                "Attestation subject does not match requested hash {}",
                expected_blake3_hash
            )));
        }

        // 4. Zero-trust check: re-hash the downloaded data to ensure it hasn't been tampered with
        // (Even if the signature is valid, the actual blob might have been swapped in transit)
        let actual_hash = blake3::hash(&payload.artifact_data).to_hex().to_string();
        if actual_hash != expected_blake3_hash {
            return Err(FederationError::HashMismatch);
        }

        // All checks passed! The CAS chunk is trusted and verified.
        Ok(Some(payload.artifact_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose};
    use ed25519_dalek::{Signer, SigningKey};
    use fish_security::slsa::{SlsaMaterial, generate_slsa_level3_statement};
    use std::collections::HashMap;

    #[test]
    fn test_global_mesh_federation_publish_and_verify() {
        let addr = "127.0.0.1:9191".parse().unwrap();
        let mesh = BananaMeshCache::new("mesh-node-1", addr);
        let mut federation = GlobalMeshFederation::new(mesh);

        let secret: [u8; 32] = core::array::from_fn(|i| i as u8 * 7);
        let signing_key = SigningKey::from_bytes(&secret);
        let verifying_key = signing_key.verifying_key();
        let pub_b64 = general_purpose::STANDARD.encode(verifying_key.as_bytes());
        federation.add_trusted_key(&pub_b64);

        let data = b"compiled object code".to_vec();
        let hash = blake3::hash(&data).to_hex().to_string();

        let statement = generate_slsa_level3_statement(
            "foo.o",
            &hash,
            "https://github.com/fish-build/official-builder",
            Some("1.0"),
            "https://fish.dev/build/v1",
            vec![SlsaMaterial {
                uri: "git+https://github.com/rust-lang/rust".to_string(),
                digest: HashMap::from([("sha1".to_string(), "abcdef".to_string())]),
            }],
            HashMap::new(),
            Some("run-123".to_string()),
        );

        let payload_bytes = statement.canonical_payload().unwrap();
        let signature = signing_key.sign(&payload_bytes);

        let signed = SignedStatement {
            statement,
            signature: general_purpose::STANDARD.encode(signature.to_bytes()),
            key_id: pub_b64.clone(),
        };

        federation
            .publish_artifact(&hash, data.clone(), signed)
            .unwrap();

        let retrieved = federation.fetch_and_verify(&hash).unwrap().unwrap();
        assert_eq!(retrieved, data);
    }
}

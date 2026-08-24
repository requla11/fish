use crate::error::SigningResult;
use crate::sbom::SbomMetadata;
use crate::signature::{ArtifactSignature, verify_signature};
use base64::{Engine as _, engine::general_purpose};
use sha2::Digest;
use std::path::Path;
use tokio::fs;

/// Verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Signature is valid
    Valid,
    /// Signature is invalid
    Invalid,
    /// Signature format error
    FormatError,
    /// Artifact not found
    ArtifactNotFound,
}

/// Verification result with details
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Overall status
    pub status: VerificationStatus,
    /// Hash match
    pub hash_match: bool,
    /// Signature valid
    pub signature_valid: bool,
    /// Metadata verified
    pub metadata_verified: bool,
    /// Verification message
    pub message: String,
}

/// Artifact verifier
pub struct ArtifactVerifier {
    /// Trusted public keys
    trusted_keys: Vec<Vec<u8>>,
}

impl ArtifactVerifier {
    /// Create a new verifier with trusted keys
    pub fn new(trusted_keys: Vec<Vec<u8>>) -> Self {
        Self { trusted_keys }
    }

    /// Create verifier with a single trusted key
    pub fn with_key(public_key: Vec<u8>) -> Self {
        Self {
            trusted_keys: vec![public_key],
        }
    }

    /// Verify an artifact
    pub async fn verify(
        &self,
        artifact_path: &Path,
        signature: &ArtifactSignature,
    ) -> SigningResult<VerificationResult> {
        if !artifact_path.exists() {
            return Ok(VerificationResult {
                status: VerificationStatus::ArtifactNotFound,
                hash_match: false,
                signature_valid: false,
                metadata_verified: false,
                message: "Artifact not found".to_string(),
            });
        }

        let artifact_content = fs::read(artifact_path).await?;

        let signature_valid = verify_signature(&artifact_content, signature)?;

        let signer_trusted = self.trusted_keys.iter().any(|key| {
            key == &general_purpose::STANDARD
                .decode(&signature.signer_public_key)
                .unwrap_or_default()
        });

        let mut hasher = sha2::Sha256::new();
        hasher.update(&artifact_content);
        let hash = hasher.finalize();
        let hash_match = general_purpose::STANDARD.encode(hash) == signature.artifact_hash;

        let status = if signature_valid && signer_trusted && hash_match {
            VerificationStatus::Valid
        } else {
            VerificationStatus::Invalid
        };

        let message = if status == VerificationStatus::Valid {
            "Artifact signature verified successfully; SBOM metadata is not \
             cryptographically bound to the signature, use \
             verify_artifact_with_metadata to check it against expectations"
                .to_string()
        } else {
            let mut reasons = Vec::new();
            if !signature_valid {
                reasons.push("signature invalid");
            }
            if !signer_trusted {
                reasons.push("signer not trusted");
            }
            if !hash_match {
                reasons.push("hash mismatch");
            }
            format!("Verification failed: {}", reasons.join(", "))
        };

        Ok(VerificationResult {
            status,
            hash_match,
            signature_valid,
            metadata_verified: false,
            message,
        })
    }

    /// Verify an artifact and additionally compare its embedded SBOM
    /// metadata against expected values.
    ///
    /// The metadata itself is not part of the signed payload, so it can only
    /// be validated against caller-supplied expectations, never asserted
    /// unconditionally.
    pub async fn verify_artifact_with_metadata(
        &self,
        artifact_path: &Path,
        signature: &ArtifactSignature,
        expected_metadata: &SbomMetadata,
    ) -> SigningResult<VerificationResult> {
        let mut result = self.verify(artifact_path, signature).await?;
        let matches = &signature.metadata == expected_metadata;
        result.metadata_verified = matches;
        if result.status == VerificationStatus::Valid && !matches {
            result.message =
                "Signature valid but SBOM metadata differs from expected values".to_string();
        }
        Ok(result)
    }

    /// Add a trusted key
    pub fn add_trusted_key(&mut self, public_key: Vec<u8>) {
        self.trusted_keys.push(public_key);
    }

    /// Get trusted keys count
    pub fn trusted_keys_count(&self) -> usize {
        self.trusted_keys.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SignatureAlgorithm;
    use crate::keypair::{KeyGenerationOptions, SigningKeyPair};
    use crate::sbom::SbomMetadata;
    use crate::signature::sign_artifact;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_artifact_verification() {
        let keypair = SigningKeyPair::generate(KeyGenerationOptions::default()).unwrap();
        let temp_file = NamedTempFile::new().unwrap();
        let content = b"test artifact content";
        fs::write(temp_file.path(), content).await.unwrap();

        let metadata = SbomMetadata::default();
        let signature = sign_artifact(
            temp_file.path(),
            &keypair,
            SignatureAlgorithm::Ed25519,
            &metadata,
        )
        .await
        .unwrap();

        let verifier = ArtifactVerifier::with_key(keypair.public_key_bytes().to_vec());
        let result = verifier.verify(temp_file.path(), &signature).await.unwrap();

        assert_eq!(result.status, VerificationStatus::Valid);
        assert!(result.hash_match);
        assert!(result.signature_valid);
    }

    #[tokio::test]
    async fn test_untrusted_signer() {
        let keypair = SigningKeyPair::generate(KeyGenerationOptions::default()).unwrap();
        let temp_file = NamedTempFile::new().unwrap();
        let content = b"test artifact content";
        fs::write(temp_file.path(), content).await.unwrap();

        let metadata = SbomMetadata::default();
        let signature = sign_artifact(
            temp_file.path(),
            &keypair,
            SignatureAlgorithm::Ed25519,
            &metadata,
        )
        .await
        .unwrap();

        let untrusted_key = SigningKeyPair::generate(KeyGenerationOptions::default())
            .unwrap()
            .public_key_bytes()
            .to_vec();
        let verifier = ArtifactVerifier::with_key(untrusted_key);
        let result = verifier.verify(temp_file.path(), &signature).await.unwrap();

        assert_eq!(result.status, VerificationStatus::Invalid);
    }

    #[tokio::test]
    async fn test_metadata_verification_against_expectations() {
        let keypair = SigningKeyPair::generate(KeyGenerationOptions::default()).unwrap();
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), b"sbom artifact").await.unwrap();

        let metadata = SbomMetadata {
            name: "fish-demo".to_string(),
            ..SbomMetadata::default()
        };
        let signature = sign_artifact(
            temp_file.path(),
            &keypair,
            SignatureAlgorithm::Ed25519,
            &metadata,
        )
        .await
        .unwrap();

        let verifier = ArtifactVerifier::with_key(keypair.public_key_bytes().to_vec());

        let matching = verifier
            .verify_artifact_with_metadata(temp_file.path(), &signature, &metadata)
            .await
            .unwrap();
        assert_eq!(matching.status, VerificationStatus::Valid);
        assert!(matching.metadata_verified);

        let tampered = SbomMetadata {
            name: "other-package".to_string(),
            ..metadata.clone()
        };
        let mismatched = verifier
            .verify_artifact_with_metadata(temp_file.path(), &signature, &tampered)
            .await
            .unwrap();
        assert!(!mismatched.metadata_verified);

        let plain = verifier.verify(temp_file.path(), &signature).await.unwrap();
        assert!(
            !plain.metadata_verified,
            "metadata must not be reported verified without expectations"
        );
    }
}

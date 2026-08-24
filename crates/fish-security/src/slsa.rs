use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Signature, SignatureError, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const IN_TOTO_STATEMENT_V1: &str = "https://in-toto.io/Statement/v1";
pub const SLSA_PROVENANCE_V1: &str = "https://slsa.dev/provenance/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InTotoStatement {
    #[serde(rename = "_type")]
    pub doc_type: String,
    pub subject: Vec<SlsaSubject>,
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    pub predicate: ProvenancePredicate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlsaSubject {
    pub name: String,
    pub digest: HashMap<String, String>,
}

/// The `https://slsa.dev/provenance/v1` predicate: a `buildDefinition`
/// describing what was asked to run and `runDetails` describing what ran.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenancePredicate {
    pub build_definition: BuildDefinition,
    pub run_details: RunDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildDefinition {
    pub build_type: String,
    pub external_parameters: HashMap<String, String>,
    pub internal_parameters: HashMap<String, String>,
    pub resolved_dependencies: Vec<SlsaMaterial>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDetails {
    pub builder: SlsaBuilder,
    pub metadata: RunMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlsaBuilder {
    pub id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunMetadata {
    pub invocation_id: Option<String>,
    pub started_on: Option<String>,
    pub finished_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlsaMaterial {
    pub uri: String,
    pub digest: HashMap<String, String>,
}

/// A statement plus its detached Ed25519 signature over the canonical JSON
/// encoding of the statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedStatement {
    pub statement: InTotoStatement,
    /// Base64 of the 64-byte Ed25519 signature.
    pub signature: String,
    /// Base64 of the signer's 32-byte Ed25519 public key.
    pub key_id: String,
}

impl InTotoStatement {
    /// Canonical byte payload that signatures commit to.
    pub fn canonical_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Check that this statement claims `name` at digest `expected_blake3`.
    pub fn verifies_subject(&self, name: &str, expected_blake3: &str) -> bool {
        self.subject.iter().any(|s| {
            s.name == name && s.digest.get("blake3").map(String::as_str) == Some(expected_blake3)
        })
    }
}

/// Generate an SLSA provenance v1 statement for one build output.
///
/// Callers own the digests: fish computes BLAKE3 itself and accepts any
/// additional algorithm digests (e.g. sha256 from toolchain output) so the
/// document stays honest about how every value was produced.
pub fn generate_statement(
    artifact_name: &str,
    blake3_hash: &str,
    builder_id: &str,
    builder_version: Option<&str>,
    build_type: &str,
    extra_digests: HashMap<String, String>,
) -> InTotoStatement {
    let mut digest = HashMap::new();
    digest.insert("blake3".to_string(), blake3_hash.to_string());
    for (algo, value) in extra_digests {
        digest.insert(algo, value);
    }

    InTotoStatement {
        doc_type: IN_TOTO_STATEMENT_V1.to_string(),
        subject: vec![SlsaSubject {
            name: artifact_name.to_string(),
            digest,
        }],
        predicate_type: SLSA_PROVENANCE_V1.to_string(),
        predicate: ProvenancePredicate {
            build_definition: BuildDefinition {
                build_type: build_type.to_string(),
                external_parameters: HashMap::from([(
                    "manifest".to_string(),
                    "fish.toml".to_string(),
                )]),
                internal_parameters: HashMap::new(),
                resolved_dependencies: Vec::new(),
            },
            run_details: RunDetails {
                builder: SlsaBuilder {
                    id: builder_id.to_string(),
                    version: builder_version.map(str::to_string),
                },
                metadata: RunMetadata {
                    invocation_id: None,
                    started_on: None,
                    finished_on: None,
                },
            },
        },
    }
}

fn verify_with_public_key(
    payload: &[u8],
    signature_b64: &str,
    public_key_b64: &str,
) -> Result<(), SignatureError> {
    let sig_bytes: [u8; 64] = general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|_| SignatureError::new())?
        .try_into()
        .map_err(|_| SignatureError::new())?;
    let key_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(public_key_b64)
        .map_err(|_| SignatureError::new())?
        .try_into()
        .map_err(|_| SignatureError::new())?;
    let verifying_key = VerifyingKey::from_bytes(&key_bytes)?;
    let signature = Signature::from_slice(&sig_bytes)?;
    verifying_key.verify(payload, &signature)
}

/// Sign a statement with an Ed25519 signing key (32-byte secret seed).
pub fn sign_statement(
    statement: &InTotoStatement,
    signing_key: &SigningKey,
) -> Result<SignedStatement, serde_json::Error> {
    let payload = statement.canonical_payload()?;
    let signature = signing_key.sign(&payload);
    Ok(SignedStatement {
        statement: statement.clone(),
        signature: general_purpose::STANDARD.encode(signature.to_bytes()),
        key_id: general_purpose::STANDARD.encode(signing_key.verifying_key().to_bytes()),
    })
}

/// Verify a signed statement end-to-end: signature over the exact serialized
/// statement bytes, then subject binding.
pub fn verify_signed_statement(
    signed: &SignedStatement,
    expected_name: &str,
    expected_blake3: &str,
) -> Result<(), String> {
    if !signed
        .statement
        .verifies_subject(expected_name, expected_blake3)
    {
        return Err(format!(
            "statement does not describe subject `{expected_name}` at the expected BLAKE3 digest"
        ));
    }
    let payload = signed
        .statement
        .canonical_payload()
        .map_err(|e| format!("canonical serialization failed: {e}"))?;
    verify_with_public_key(&payload, &signed.signature, &signed.key_id)
        .map_err(|e| format!("signature verification failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn sample_statement() -> InTotoStatement {
        generate_statement(
            "target/release/fish",
            "9f86d081884c7d659a2f",
            "https://github.com/requla11/fish",
            Some("0.5.0"),
            "https://fish.build/tasks/v1",
            HashMap::new(),
        )
    }

    #[test]
    fn test_statement_matches_in_toto_v1_shape() {
        let stmt = sample_statement();
        assert_eq!(stmt.doc_type, IN_TOTO_STATEMENT_V1);
        assert_eq!(stmt.predicate_type, SLSA_PROVENANCE_V1);

        let json = serde_json::to_value(&stmt).unwrap();
        assert_eq!(
            json["predicate"]["buildDefinition"]["buildType"],
            "https://fish.build/tasks/v1"
        );
        assert!(
            json["predicate"]["runDetails"]["builder"]["id"]
                .as_str()
                .unwrap()
                .starts_with("https://")
        );
    }

    #[test]
    fn test_subject_verification_accepts_and_rejects() {
        let stmt = sample_statement();
        assert!(stmt.verifies_subject("target/release/fish", "9f86d081884c7d659a2f"));
        assert!(!stmt.verifies_subject("other.bin", "9f86d081884c7d659a2f"));
        assert!(!stmt.verifies_subject("target/release/fish", "wrong"));
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let secret: [u8; 32] = core::array::from_fn(|i| i as u8 * 7);
        let signing_key = SigningKey::from_bytes(&secret);
        let stmt = sample_statement();

        let signed = sign_statement(&stmt, &signing_key).unwrap();
        assert!(
            verify_signed_statement(&signed, "target/release/fish", "9f86d081884c7d659a2f").is_ok()
        );
    }

    #[test]
    fn test_tampered_statement_fails_verification() {
        let secret: [u8; 32] = core::array::from_fn(|i| (i as u32 * 11 % 256) as u8);
        let signing_key = SigningKey::from_bytes(&secret);
        let mut signed = sign_statement(&sample_statement(), &signing_key).unwrap();

        signed
            .statement
            .predicate
            .build_definition
            .external_parameters
            .insert("injected".to_string(), "parameter".to_string());

        let err = verify_signed_statement(&signed, "target/release/fish", "9f86d081884c7d659a2f")
            .unwrap_err();
        assert!(err.contains("signature verification failed"), "got: {err}");
    }
}

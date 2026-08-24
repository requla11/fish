//! Signature verification gate for remote cache artifacts.
//!
//! Wraps any [`RemoteCacheClient`] so that artifacts pulled from a remote
//! peer are only accepted when their Ed25519 signature verifies against one
//! of the configured trusted keys. Unsigned or tampered payloads are refused
//! (or downgraded per policy) instead of silently poisoning the local CAS.

use std::collections::HashSet;

use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Signature, SignatureError, SigningKey, VerifyingKey};
use ed25519_dalek::{Signer, Verifier};

use crate::{RemoteCacheClient, RemoteCacheError};

/// What happens when an artifact fails signature verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GatePolicy {
    /// Refuse the artifact — callers see `None` from `get_artifact`.
    #[default]
    Refuse,
    /// Still return the payload (transition mode for gradual rollout).
    WarnOnly,
}

/// Wire format: `[artifact] || 0x00 || [sig_64] || [key_32]`
///
/// Ed25519 always produces exactly 64-byte signatures and 32-byte keys,
/// so the trailer has fixed width (97 bytes). Parsing from the tail is
/// unambiguous regardless of artifact content.
pub fn pack_signed(artifact: &[u8], signature: &[u8], public_key: &[u8]) -> Vec<u8> {
    debug_assert_eq!(signature.len(), 64);
    debug_assert_eq!(public_key.len(), 32);
    let mut out = Vec::with_capacity(artifact.len() + 1 + 64 + 32);
    out.extend_from_slice(artifact);
    out.push(0x00);
    out.extend_from_slice(signature);
    out.extend_from_slice(public_key);
    out
}

/// Attempt to split a wire blob back into `(artifact, signature, public_key)`.
pub fn unpack_signed(wire: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    if wire.len() < 97 {
        return None;
    }
    let sep = wire.len() - 97;
    if wire[sep] != 0x00 {
        return None;
    }
    let sig_start = sep + 1;
    let key_start = sig_start + 64;
    Some((
        wire[..sep].to_vec(),
        wire[sig_start..sig_start + 64].to_vec(),
        wire[key_start..].to_vec(),
    ))
}

fn verify_signature(
    artifact: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), SignatureError> {
    let key: [u8; 32] = public_key.try_into().map_err(|_| SignatureError::new())?;
    let vk = VerifyingKey::from_bytes(&key)?;
    let sig = Signature::from_slice(signature)?;
    vk.verify(artifact, &sig)
}

/// Wrap a [`RemoteCacheClient`] to enforce Ed25519 artifact signatures.
///
/// `put_artifact` signs outgoing blobs with `signing_seed`; `get_artifact`
/// verifies incoming blobs against `trusted_keys_b64` before accepting them.
#[derive(Debug)]
pub struct SignedArtifactGate<I> {
    inner: I,
    signing_seed: [u8; 32],
    /// Base64-encoded Ed25519 public keys that artifacts must verify against.
    trusted_keys_b64: HashSet<String>,
    policy: GatePolicy,
}

impl<I: RemoteCacheClient> SignedArtifactGate<I> {
    pub fn new(
        inner: I,
        signing_seed: [u8; 32],
        trusted_keys_b64: HashSet<String>,
        policy: GatePolicy,
    ) -> Self {
        Self {
            inner,
            signing_seed,
            trusted_keys_b64,
            policy,
        }
    }

    fn sign_and_pack(&self, artifact: &[u8]) -> Result<Vec<u8>, RemoteCacheError> {
        let signing_key = SigningKey::from_bytes(&self.signing_seed);
        let signature = signing_key.sign(artifact);
        let public_key = signing_key.verifying_key().to_bytes();
        Ok(pack_signed(artifact, &signature.to_bytes(), &public_key))
    }

    fn unpack_and_verify(&self, wire: &[u8]) -> Result<Vec<u8>, RemoteCacheError> {
        let err = |msg: String| RemoteCacheError::Protocol(msg);

        let (artifact, signature, public_key) =
            unpack_signed(wire).ok_or_else(|| err("malformed signed envelope".into()))?;

        // Verify cryptographic integrity first.
        verify_signature(&artifact, &signature, &public_key)
            .map_err(|e| err(format!("Ed25519 verification failed: {e}")))?;

        // Then check trust.
        let key_b64 = general_purpose::STANDARD.encode(&public_key);
        if self.trusted_keys_b64.contains(&key_b64) {
            return Ok(artifact);
        }

        match self.policy {
            GatePolicy::Refuse => Err(err(format!(
                "signing key `{key_b64}` is not in the trusted set"
            ))),
            GatePolicy::WarnOnly => Ok(artifact),
        }
    }
}

impl<I: RemoteCacheClient> RemoteCacheClient for SignedArtifactGate<I> {
    fn get_fingerprint(&self, key: &str) -> Result<Option<String>, RemoteCacheError> {
        self.inner.get_fingerprint(key)
    }

    fn put_fingerprint(&self, key: &str, fingerprint: &str) -> Result<(), RemoteCacheError> {
        self.inner.put_fingerprint(key, fingerprint)
    }

    fn get_artifact(&self, key: &str) -> Result<Option<Vec<u8>>, RemoteCacheError> {
        match self.inner.get_artifact(key)? {
            None => Ok(None),
            Some(wire) => {
                let artifact = self.unpack_and_verify(&wire).inspect_err(|_| {
                    if self.policy == GatePolicy::Refuse {
                        eprintln!("warning: refused unsigned/tampered artifact for `{key}`");
                    }
                })?;
                Ok(Some(artifact))
            }
        }
    }

    fn put_artifact(&self, key: &str, data: &[u8]) -> Result<(), RemoteCacheError> {
        let signed_wire = self.sign_and_pack(data)?;
        self.inner.put_artifact(key, &signed_wire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryRemoteCache;
    use std::collections::HashSet;

    fn gate(
        seed: [u8; 32],
        trusted: HashSet<String>,
        policy: GatePolicy,
    ) -> SignedArtifactGate<InMemoryRemoteCache> {
        SignedArtifactGate::new(InMemoryRemoteCache::new(), seed, trusted, policy)
    }

    fn b64(bytes: &[u8]) -> String {
        general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        let artifact = b"hello cache blob".to_vec();
        let sig = [0xABu8; 64].to_vec();
        let key = [0xCDu8; 32].to_vec();

        let wire = pack_signed(&artifact, &sig, &key);
        let (out_artifact, out_sig, out_key) = unpack_signed(&wire).unwrap();

        assert_eq!(out_artifact, artifact);
        assert_eq!(out_sig, sig);
        assert_eq!(out_key, key);
    }

    #[test]
    fn test_malformed_envelopes_return_none() {
        // Too short
        assert_eq!(unpack_signed(&[]), None);
        // No separator
        assert_eq!(unpack_signed(b"no separator here"), None);
        // Separator but truncated lengths
        assert_eq!(unpack_signed(&[0x61, 0x00, 0xFF]), None);
    }

    #[test]
    fn test_signed_put_get_roundtrip_with_trusted_key() {
        let seed = [42u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let public_b64 = b64(signing_key.verifying_key().to_bytes().as_slice());

        let mut trusted = HashSet::new();
        trusted.insert(public_b64);

        let g = gate(seed, trusted, GatePolicy::Refuse);
        g.put_artifact("key_a", b"cached data").unwrap();

        let result = g.get_artifact("key_a").unwrap();
        assert_eq!(result.as_deref(), Some(b"cached data".as_slice()));
    }

    #[test]
    fn test_refuse_policy_rejects_untrusted_signer() {
        let writer_seed = [1u8; 32];
        let reader_seed = [2u8; 32];

        // Reader trusts only its own key (not the writer's).
        let reader_key = SigningKey::from_bytes(&reader_seed);
        let mut trusted = HashSet::new();
        trusted.insert(b64(reader_key.verifying_key().to_bytes().as_slice()));

        let writer = gate(writer_seed, HashSet::new(), GatePolicy::Refuse);
        writer.put_artifact("key_b", b"poisoned").unwrap();

        let reader = SignedArtifactGate::new(
            InMemoryRemoteCache::new(),
            reader_seed,
            trusted,
            GatePolicy::Refuse,
        );
        // Copy the raw wire from the backing store to bypass writer signing.
        let wire = writer.inner.get_artifact("key_b").unwrap().unwrap();
        reader.inner.put_artifact("key_b", &wire).unwrap();

        let result = reader.get_artifact("key_b");
        assert!(result.is_err(), "untrusted signer must be refused");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not in the trusted set")
        );
    }

    #[test]
    fn test_warn_only_policy_passes_untrusted_but_returns_data() {
        let writer_seed = [1u8; 32];
        let reader_seed = [2u8; 32];

        let reader_key = SigningKey::from_bytes(&reader_seed);
        let mut trusted = HashSet::new();
        trusted.insert(b64(reader_key.verifying_key().to_bytes().as_slice()));

        let writer = gate(writer_seed, HashSet::new(), GatePolicy::WarnOnly);
        writer.put_artifact("key_c", b"transition data").unwrap();

        let wire = writer.inner.get_artifact("key_c").unwrap().unwrap();
        let reader = SignedArtifactGate::new(
            InMemoryRemoteCache::new(),
            reader_seed,
            trusted,
            GatePolicy::WarnOnly,
        );
        reader.inner.put_artifact("key_c", &wire).unwrap();

        let result = reader.get_artifact("key_c");
        assert!(
            result
                .as_ref()
                .is_ok_and(|d| d.as_deref() == Some(b"transition data".as_slice())),
            "warn-only must pass untrusted data through"
        );
    }

    #[test]
    fn test_tampered_artifact_fails_verification_even_warn_only() {
        let seed = [7u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let public_b64 = b64(signing_key.verifying_key().to_bytes().as_slice());
        let mut trusted = HashSet::new();
        trusted.insert(public_b64);

        let artifact = b"original".to_vec();
        let signature = signing_key.sign(&artifact).to_bytes();
        let public = signing_key.verifying_key().to_bytes();

        // Tamper with the artifact bytes after signing.
        let mut tampered = artifact.clone();
        tampered[0] ^= 0xFF;
        let wire = pack_signed(&tampered, &signature, &public);

        let g = gate(seed, trusted, GatePolicy::WarnOnly);
        g.inner.put_artifact("key_d", &wire).unwrap();

        let result = g.get_artifact("key_d");
        assert!(
            result.is_err(),
            "tampered payload must fail even in warn-only"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("verification failed")
        );
    }
}

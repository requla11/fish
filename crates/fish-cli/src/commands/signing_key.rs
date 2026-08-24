use std::process::ExitCode;

/// `fish signing-key`: derive the Ed25519 public key from
/// `FISH_SIGNING_SEED` (64-char hex) so operators can pin it in
/// `FISH_TRUSTED_KEYS` and consumers can verify artifacts offline.
///
/// The seed itself is never printed.
pub fn run_signing_key() -> ExitCode {
    let seed_hex = match std::env::var("FISH_SIGNING_SEED") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("error: FISH_SIGNING_SEED is not set.");
            eprintln!("Set it to the 64-char hex seed used for artifact signing.");
            return ExitCode::FAILURE;
        }
    };

    let mut seed = [0u8; 32];
    if hex::decode_to_slice(seed_hex.trim(), &mut seed).is_err() {
        eprintln!("error: FISH_SIGNING_SEED must decode to exactly 32 bytes (64 hex chars).");
        return ExitCode::FAILURE;
    }

    use ed25519_dalek::Signer;
    let signing = ed25519_dalek::SigningKey::from_bytes(&seed);
    let verifying = signing.verifying_key();
    println!("{}", hex::encode(verifying.as_bytes()));
    ExitCode::SUCCESS
}

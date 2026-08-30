use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;

use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::SigningKey;

use crate::args::{AttestArgs, VerifyArgs};
use crate::attestation;
use crate::utils::resolve_start_dir;

fn resolve_signing_key(key_path: Option<&Path>, seed_str: Option<&str>) -> Option<SigningKey> {
    if let Some(path) = key_path
        && let Ok(content) = std::fs::read(path)
    {
        if content.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&content);
            return Some(SigningKey::from_bytes(&arr));
        }
        if let Ok(s) = std::str::from_utf8(&content)
            && let Ok(decoded) = general_purpose::STANDARD.decode(s.trim())
            && let Ok(arr) = decoded.try_into()
        {
            return Some(SigningKey::from_bytes(&arr));
        }
    }
    if let Some(seed) = seed_str {
        let hash = blake3::hash(seed.as_bytes());
        return Some(SigningKey::from_bytes(hash.as_bytes()));
    }
    if let Ok(env_seed) = std::env::var("FISH_SIGNING_SEED") {
        let hash = blake3::hash(env_seed.as_bytes());
        return Some(SigningKey::from_bytes(hash.as_bytes()));
    }
    None
}

pub fn run_attest(args: AttestArgs) -> ExitCode {
    let start_dir = match resolve_start_dir(args.path.as_deref()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let target_dir = start_dir.join("target");
    let mut outputs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                outputs.push(path);
            }
        }
    }

    let maybe_key = resolve_signing_key(args.key.as_deref(), args.seed.as_deref());

    match attestation::AttestationEngine::generate_attestation(&start_dir, &outputs) {
        Ok(attestation) => {
            match attestation::AttestationEngine::save_attestation(&start_dir, &attestation) {
                Ok(saved_path) => {
                    println!(
                        "🔒 SLSA provenance attestation generated: {}",
                        saved_path.display()
                    );
                    println!("   Merkle Root: {}", attestation.merkle_root);

                    let stmt_dir = start_dir.join(".fish").join("attestation");
                    let _ = std::fs::create_dir_all(&stmt_dir);

                    for output in &outputs {
                        let bytes = match std::fs::read(output) {
                            Ok(b) => b,
                            Err(_) => continue,
                        };
                        let blake3_hash = blake3::hash(&bytes).to_hex().to_string();
                        let name = output
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        let statement = if args.slsa_l3 {
                            let mut env_params = HashMap::new();
                            env_params.insert("os".to_string(), std::env::consts::OS.to_string());
                            env_params
                                .insert("arch".to_string(), std::env::consts::ARCH.to_string());
                            fish_security::slsa::generate_slsa_level3_statement(
                                &name,
                                &blake3_hash,
                                "https://github.com/requla11/fish",
                                Some(env!("CARGO_PKG_VERSION")),
                                "https://fish.build/tasks/v1",
                                vec![],
                                env_params,
                                None,
                            )
                        } else {
                            fish_security::slsa::generate_statement(
                                &name,
                                &blake3_hash,
                                "https://github.com/requla11/fish",
                                Some(env!("CARGO_PKG_VERSION")),
                                "https://fish.build/tasks/v1",
                                Default::default(),
                            )
                        };

                        let stmt_path = stmt_dir.join(format!("{name}.intoto.jsonl"));
                        if let Ok(json) = serde_json::to_string_pretty(&statement)
                            && std::fs::write(&stmt_path, json).is_ok()
                        {
                            println!("   In-toto statement: {}", stmt_path.display());
                        }

                        if let Some(signing_key) = &maybe_key {
                            let signed_path = stmt_dir.join(format!("{name}.intoto.signed.json"));
                            if let Ok(saved) = fish_security::slsa::sign_statement_file(
                                &stmt_path,
                                signing_key,
                                &signed_path,
                            ) {
                                let key_id = general_purpose::STANDARD
                                    .encode(signing_key.verifying_key().to_bytes());
                                println!(
                                    "   Signed statement: {} (key_id: {})",
                                    saved.display(),
                                    key_id
                                );
                            }
                        }
                    }

                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: failed to save attestation: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("error: failed to generate attestation: {e}");
            ExitCode::FAILURE
        }
    }
}

pub fn run_verify(args: VerifyArgs) -> ExitCode {
    let start_dir = match resolve_start_dir(args.path.as_deref()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let target_dir = start_dir.join("target");
    let file_path = &args.attestation_file;

    if let Ok(content) = std::fs::read_to_string(file_path) {
        if let Ok(signed) = serde_json::from_str::<fish_security::slsa::SignedStatement>(&content) {
            let mut trusted_keys = Vec::new();
            if let Some(pk) = &args.public_key {
                trusted_keys.push(pk.clone());
            }
            if let Some(tk_path) = &args.trusted_keys
                && let Ok(tk_content) = std::fs::read_to_string(tk_path)
            {
                for line in tk_content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        trusted_keys.push(trimmed.to_string());
                    }
                }
            }

            for subject in &signed.statement.subject {
                if let Some(expected_blake3) = subject.digest.get("blake3")
                    && let Err(e) = fish_security::slsa::verify_statement_file(
                        file_path,
                        &subject.name,
                        expected_blake3,
                        &trusted_keys,
                    )
                {
                    eprintln!("✗ Signed SLSA Verification Failed: {e}");
                    return ExitCode::FAILURE;
                }
            }

            if args.slsa_l3
                && let Err(e) =
                    fish_security::slsa::verify_slsa_level3_compliance(&signed.statement)
            {
                eprintln!("✗ SLSA Level 3 Compliance Failed: {e}");
                return ExitCode::FAILURE;
            }

            println!("✓ Signed SLSA Provenance Verified: Signature valid & tamper-free.");
            return ExitCode::SUCCESS;
        }

        if let Ok(statement) =
            serde_json::from_str::<fish_security::slsa::InTotoStatement>(&content)
        {
            if args.slsa_l3
                && let Err(e) = fish_security::slsa::verify_slsa_level3_compliance(&statement)
            {
                eprintln!("✗ SLSA Level 3 Compliance Failed: {e}");
                return ExitCode::FAILURE;
            }
            println!("✓ In-toto Statement Verified.");
            return ExitCode::SUCCESS;
        }
    }

    match attestation::AttestationEngine::verify_attestation(file_path, &target_dir) {
        Ok(true) => {
            println!("✓ SLSA Provenance Verified: Artifacts are pristine and tamper-free.");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            eprintln!("✗ SLSA Verification Failed: Artifacts modified or checksum mismatch!");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("error: verification failed: {e}");
            ExitCode::FAILURE
        }
    }
}

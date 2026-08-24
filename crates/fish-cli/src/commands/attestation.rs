use std::process::ExitCode;

use crate::args::{AttestArgs, VerifyArgs};
use crate::attestation;
use crate::utils::resolve_start_dir;

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

    match attestation::AttestationEngine::generate_attestation(&start_dir, &outputs) {
        Ok(attestation) => {
            match attestation::AttestationEngine::save_attestation(&start_dir, &attestation) {
                Ok(saved_path) => {
                    println!(
                        "🔒 SLSA provenance attestation generated: {}",
                        saved_path.display()
                    );
                    println!("   Merkle Root: {}", attestation.merkle_root);

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

                        let statement = fish_security::slsa::generate_statement(
                            &name,
                            &blake3_hash,
                            "https://github.com/requla11/fish",
                            Some(env!("CARGO_PKG_VERSION")),
                            "https://fish.build/tasks/v1",
                            Default::default(),
                        );

                        let stmt_dir = start_dir.join(".fish").join("attestation");
                        let stmt_path = stmt_dir.join(format!("{name}.intoto.jsonl"));
                        if let Ok(json) = serde_json::to_string_pretty(&statement)
                            && std::fs::create_dir_all(&stmt_dir).is_ok()
                            && std::fs::write(&stmt_path, json).is_ok()
                        {
                            println!("   In-toto statement: {}", stmt_path.display());
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
    match attestation::AttestationEngine::verify_attestation(&args.attestation_file, &target_dir) {
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

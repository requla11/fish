use fish_cache::LocalCache;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::args::WhyArgs;
use crate::nl_query::{CacheQuery, default_cache_root, explain_rebuild, parse_query};

pub fn run_why(args: WhyArgs) -> ExitCode {
    match parse_query(&args.target) {
        CacheQuery::WhyRebuilt { target } => answer_why_rebuilt(&target),
        CacheQuery::DriftSummary => answer_drift_summary(),
        CacheQuery::Stats => answer_cache_stats(),
        CacheQuery::Unknown => {
            let trimmed = args.target.trim();
            if !trimmed.is_empty() && !trimmed.contains(' ') {
                answer_why_rebuilt(trimmed)
            } else {
                println!("Unrecognized question or target. Try:");
                println!("  fish why <target>");
                println!("  fish why \"why did <target> rebuild\"");
                println!("  fish why \"what changed\"");
                println!("  fish why \"cache stats\"");
                ExitCode::FAILURE
            }
        }
    }
}

fn answer_why_rebuilt(target: &str) -> ExitCode {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cache_root = default_cache_root(&project_root);
    let explanation = explain_rebuild(&cache_root, target);

    if let Some(ref report) = explanation.detailed_report {
        print!("{report}");
    } else {
        println!("=== Rebuild explanation for `{}` ===", explanation.target);
        match explanation.cached_fingerprint {
            Some(fp) => {
                let short_fp = &fp[..fp.len().min(16)];
                println!("Cached fingerprint: {}...", short_fp);
                println!("Verdict: record exists — the rebuild was triggered by fingerprint drift");
                println!("(source hash, toolchain flags, or env vars changed since this record).");
                println!("Compare against the current inputs with `fish build --verbose`.");
            }
            None => {
                println!(
                    "No cached fingerprint record found for this target in {}.",
                    cache_root.display()
                );
                println!("The next build will be treated as a cold miss and populate the record.");
            }
        }
    }
    ExitCode::SUCCESS
}

fn answer_drift_summary() -> ExitCode {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cache_root = default_cache_root(&project_root);
    let cache = match LocalCache::new(&cache_root) {
        Ok(c) => c,
        Err(_) => {
            println!("Local cache unavailable at {}.", cache_root.display());
            return ExitCode::FAILURE;
        }
    };

    let manifests = cache.list_manifests();
    if manifests.is_empty() {
        println!("No recorded task manifests found in cache.");
        println!("Run `fish build` first to populate the cache and generate manifests.");
        return ExitCode::SUCCESS;
    }

    println!(
        "=== Cache drift summary (tracking {} targets) ===",
        manifests.len()
    );
    let mut drifted_count = 0;
    for m in &manifests {
        let diff = m.diff_against_working_tree(&project_root);
        if diff.verdict == fish_cache::ManifestVerdict::Drifted {
            drifted_count += 1;
            let mut reasons = Vec::new();
            if !diff.modified_files.is_empty() {
                reasons.push(format!("{} modified file(s)", diff.modified_files.len()));
            }
            if !diff.added_files.is_empty() {
                reasons.push(format!("{} added file(s)", diff.added_files.len()));
            }
            if !diff.removed_files.is_empty() {
                reasons.push(format!("{} removed file(s)", diff.removed_files.len()));
            }
            if !diff.changed_envs.is_empty() {
                reasons.push(format!("{} env change(s)", diff.changed_envs.len()));
            }
            println!("  ~ {}: {}", m.label, reasons.join(", "));
        }
    }

    if drifted_count == 0 {
        println!(
            "All {} cached targets match the current working tree.",
            manifests.len()
        );
    } else {
        println!(
            "\nTotal drifted targets: {}/{}.",
            drifted_count,
            manifests.len()
        );
        println!("Run `fish why <target>` for deep inspection of any specific target.");
    }
    ExitCode::SUCCESS
}

fn answer_cache_stats() -> ExitCode {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cache_root = default_cache_root(&project_root);
    let cache = match LocalCache::new(&cache_root) {
        Ok(c) => c,
        Err(_) => {
            println!("Local cache unavailable at {}.", cache_root.display());
            return ExitCode::FAILURE;
        }
    };

    let stats = cache.stats();
    let manifests = cache.list_manifests();
    println!("=== Fish Cache Statistics ===");
    println!("Cache directory: {}", cache_root.display());
    println!("Runtime hits: {}", stats.hits());
    println!("Runtime misses: {}", stats.misses());
    println!("Runtime errors: {}", stats.errors());
    println!("Recorded manifests: {}", manifests.len());

    let disk_stats = cache.disk_stats();
    println!("Fingerprint records on disk: {}", disk_stats.record_count);
    println!("CAS objects count: {}", disk_stats.object_count);
    println!(
        "Total cache size: {} bytes ({:.2} MB)",
        disk_stats.total_bytes,
        disk_stats.total_bytes as f64 / (1024.0 * 1024.0)
    );
    ExitCode::SUCCESS
}

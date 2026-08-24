use std::path::PathBuf;
use std::process::ExitCode;

use crate::args::WhyArgs;
use crate::nl_query::{CacheQuery, default_cache_root, explain_rebuild, parse_query};

pub fn run_why(args: WhyArgs) -> ExitCode {
    match parse_query(&args.target) {
        CacheQuery::WhyRebuilt { target } => answer_why_rebuilt(&target),
        CacheQuery::DriftSummary => {
            println!("Drift summary requires a fingerprint comparison run.");
            println!("Run `fish build` first, then ask e.g.: fish why \"why did core rebuild\"");
            ExitCode::SUCCESS
        }
        CacheQuery::Stats => {
            println!("Cache statistics are reported at the end of every `fish build` run.");
            ExitCode::SUCCESS
        }
        CacheQuery::Unknown => {
            println!("Unrecognized question. Try:");
            println!("  fish why \"why did <target> rebuild\"");
            println!("  fish why \"what changed\"");
            println!("  fish why \"cache stats\"");
            ExitCode::FAILURE
        }
    }
}

fn answer_why_rebuilt(target: &str) -> ExitCode {
    let cache_root =
        default_cache_root(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let explanation = explain_rebuild(&cache_root, target);

    println!("=== Rebuild explanation for `{}` ===", explanation.target);
    match explanation.cached_fingerprint {
        Some(fp) => {
            println!("Cached fingerprint: {}…", &fp[..fp.len().min(16)]);
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
    ExitCode::SUCCESS
}

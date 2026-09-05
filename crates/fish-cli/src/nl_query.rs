use fish_cache::{LocalCache, ManifestDiff, ManifestVerdict};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum CacheQuery {
    WhyRebuilt { target: String },
    DriftSummary,
    Stats,
    Unknown,
}

pub fn parse_query(input: &str) -> CacheQuery {
    let normalized = input.to_lowercase();
    let words: Vec<&str> = normalized.split_whitespace().collect();

    if let Some(pos) = words.iter().position(|w| *w == "why")
        && let Some(rb) = words.iter().position(|w| {
            matches!(
                *w,
                "rebuild" | "rebuilt" | "recompiled" | "recompiles" | "recompile"
            )
        })
    {
        let start = pos + 1;
        let end = rb.min(words.len());
        if end > start {
            let target_words: Vec<&str> = words[start..end]
                .iter()
                .copied()
                .filter(|w| !matches!(*w, "did" | "does" | "the"))
                .collect();
            if !target_words.is_empty() {
                return CacheQuery::WhyRebuilt {
                    target: target_words.join(" "),
                };
            }
        }
    }

    if normalized.contains("what changed") || normalized.contains("drift") {
        return CacheQuery::DriftSummary;
    }
    if normalized.contains("hit")
        || normalized.contains("miss")
        || normalized.contains("statistic")
        || normalized.contains("stats")
    {
        return CacheQuery::Stats;
    }
    CacheQuery::Unknown
}

#[derive(Debug, Clone, PartialEq)]
pub struct RebuildExplanation {
    pub target: String,
    pub cached_fingerprint: Option<String>,
    pub artifact_hash: Option<String>,
    pub verdict: &'static str,
    pub detailed_report: Option<String>,
    pub diff: Option<ManifestDiff>,
}

pub fn explain_rebuild(cache_root: &Path, target: &str) -> RebuildExplanation {
    let cache = match LocalCache::new(cache_root) {
        Ok(c) => c,
        Err(_) => {
            return RebuildExplanation {
                target: target.to_string(),
                cached_fingerprint: None,
                artifact_hash: None,
                verdict: "cache-unavailable",
                detailed_report: None,
                diff: None,
            };
        }
    };

    if let Some(manifest) = cache.find_manifest_by_target(target) {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let diff = manifest.diff_against_working_tree(&project_root);
        let detailed_report = Some(diff.format_explanation());
        let verdict = match diff.verdict {
            ManifestVerdict::ColdMiss => "no-record",
            ManifestVerdict::ExactMatch => "exact-match",
            ManifestVerdict::Drifted => "drifted",
        };
        return RebuildExplanation {
            target: manifest.label.clone(),
            cached_fingerprint: Some(manifest.total_fingerprint.clone()),
            artifact_hash: cache.artifact_hash(&manifest.key),
            verdict,
            detailed_report,
            diff: Some(diff),
        };
    }

    match cache.get(target) {
        Some(fingerprint) => {
            let artifact_hash = cache.artifact_hash(target);
            RebuildExplanation {
                target: target.to_string(),
                verdict: "found-record",
                cached_fingerprint: Some(fingerprint),
                artifact_hash,
                detailed_report: None,
                diff: None,
            }
        }
        None => {
            let cold_diff = ManifestDiff::cold_miss(target);
            let detailed_report = Some(cold_diff.format_explanation());
            RebuildExplanation {
                target: target.to_string(),
                cached_fingerprint: None,
                artifact_hash: None,
                verdict: "no-record",
                detailed_report,
                diff: Some(cold_diff),
            }
        }
    }
}

pub fn default_cache_root(project_root: &Path) -> PathBuf {
    let local = project_root.join(".fish").join("cache");
    if local.exists() {
        return local;
    }
    if let Ok(cache) = LocalCache::default_location() {
        return cache.root().to_path_buf();
    }
    local
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_why_rebuild_question() {
        assert_eq!(
            parse_query("why did core rebuild"),
            CacheQuery::WhyRebuilt {
                target: "core".into()
            }
        );
        assert_eq!(
            parse_query("why does the parser recompile"),
            CacheQuery::WhyRebuilt {
                target: "parser".into()
            }
        );
    }

    #[test]
    fn parses_drift_and_stats() {
        assert_eq!(
            parse_query("what changed since yesterday"),
            CacheQuery::DriftSummary
        );
        assert_eq!(parse_query("show me the cache stats"), CacheQuery::Stats);
        assert_eq!(parse_query("how many misses today?"), CacheQuery::Stats);
    }

    #[test]
    fn unknown_falls_through() {
        assert_eq!(parse_query("hello world"), CacheQuery::Unknown);
    }

    #[test]
    fn why_without_target_is_unknown() {
        assert_eq!(parse_query("why"), CacheQuery::Unknown);
    }

    #[test]
    fn explain_reports_missing_record() {
        let dir = tempfile::tempdir().unwrap();
        let explanation = explain_rebuild(dir.path(), "no-such-target");
        assert_eq!(explanation.verdict, "no-record");
        assert!(explanation.cached_fingerprint.is_none());
    }

    #[test]
    fn explain_reports_manifest_diff() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LocalCache::new(dir.path()).unwrap();
        let manifest = fish_cache::TaskManifest {
            key: "key-core".to_string(),
            label: "core".to_string(),
            command: "rustc".to_string(),
            args: vec!["src/lib.rs".to_string()],
            env: std::collections::BTreeMap::new(),
            inputs: vec![fish_cache::FileDigest {
                path: "nonexistent.rs".to_string(),
                hash: "0000".to_string(),
                size: 0,
            }],
            upstream_deps: std::collections::BTreeMap::new(),
            total_fingerprint: "fp-core".to_string(),
            stored_at: 100,
        };
        cache.put_manifest(&manifest).unwrap();
        let explanation = explain_rebuild(dir.path(), "core");
        assert_eq!(explanation.verdict, "drifted");
        assert_eq!(explanation.target, "core");
        assert!(explanation.detailed_report.is_some());
    }
}

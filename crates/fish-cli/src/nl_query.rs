use fish_cache::LocalCache;
use std::path::{Path, PathBuf};

/// A parsed natural-language query about cache behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum CacheQuery {
    /// "why did <target> rebuild" / "why did the build recompile X"
    WhyRebuilt { target: String },
    /// "what changed since last build" / "show drift"
    DriftSummary,
    /// "how many hits/misses" / "cache stats"
    Stats,
    /// Anything else.
    Unknown,
}

/// Rule-based NL parsing — no ML dependency.
///
/// Recognizes a small grammar of question templates; anything unmatched
/// falls through to [`CacheQuery::Unknown`] so callers can show usage.
pub fn parse_query(input: &str) -> CacheQuery {
    let normalized = input.to_lowercase();
    let words: Vec<&str> = normalized.split_whitespace().collect();

    if let Some(pos) = words.iter().position(|w| *w == "why") {
        // Collect target tokens after "why did/does ... rebuild/recompile".
        if let Some(rb) = words.iter().position(|w| {
            matches!(
                *w,
                "rebuild" | "rebuilt" | "recompiled" | "recompiles" | "recompile"
            )
        }) {
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

/// Answer produced by consulting real fingerprint records on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct RebuildExplanation {
    pub target: String,
    pub cached_fingerprint: Option<String>,
    pub artifact_hash: Option<String>,
    pub verdict: &'static str,
}

/// Look up the stored fingerprint for `target` in the project's local cache.
pub fn explain_rebuild(cache_root: &Path, target: &str) -> RebuildExplanation {
    let cache = match LocalCache::new(cache_root) {
        Ok(c) => c,
        Err(_) => {
            return RebuildExplanation {
                target: target.to_string(),
                cached_fingerprint: None,
                artifact_hash: None,
                verdict: "cache-unavailable",
            };
        }
    };
    match cache.get(target) {
        Some(fingerprint) => RebuildExplanation {
            target: target.to_string(),
            verdict: "found-record",
            cached_fingerprint: Some(fingerprint),
            artifact_hash: None,
        },
        None => RebuildExplanation {
            target: target.to_string(),
            cached_fingerprint: None,
            artifact_hash: None,
            verdict: "no-record",
        },
    }
}

/// Default fish state directory for a project.
pub fn default_cache_root(project_root: &Path) -> PathBuf {
    project_root.join(".fish").join("cache")
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
}

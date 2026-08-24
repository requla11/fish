use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::otel::{OtelSpan, OtelTracer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeStats {
    pub source_workers: usize,
    pub total_spans: usize,
    pub dropped_duplicates: usize,
    pub re_parented_orphans: usize,
    pub synthesized_root: bool,
}

/// Merge spans collected by multiple executors of one build into a single
/// coherent trace.
///
/// Workers each record spans under their own trace id unless launched through
/// OTLP propagation. Aggregation reconciles three situations:
///
/// 1. identical spans arriving twice (retries, at-least-once export) are
///    deduplicated on `(trace_id, span_id)`;
/// 2. every surviving span adopts the canonical trace id of the
///    earliest-starting worker;
/// 3. spans whose parent did not survive the merge are re-parented onto the
///    earliest surviving root; when no root exists at all, a synthetic
///    `build.aggregated` root is created so the result stays one connected
///    trace.
///
/// Nothing is ever dropped silently: only exact duplicates are removed and
/// every adjustment is reported in [`MergeStats`].
pub fn merge_worker_traces(tracers: &[&OtelTracer]) -> (OtelTracer, MergeStats) {
    let mut seen: HashMap<(String, String), ()> = HashMap::new();
    let mut all_spans: Vec<OtelSpan> = Vec::new();
    let mut dropped_duplicates = 0usize;

    for tracer in tracers {
        for span in tracer.spans() {
            let key = (span.trace_id.clone(), span.span_id.clone());
            if seen.insert(key, ()).is_some() {
                dropped_duplicates += 1;
                continue;
            }
            all_spans.push(span);
        }
    }

    if all_spans.is_empty() {
        return (
            OtelTracer::with_trace_id("fish-build", "0".repeat(32)),
            MergeStats {
                source_workers: tracers.len(),
                total_spans: 0,
                dropped_duplicates,
                re_parented_orphans: 0,
                synthesized_root: false,
            },
        );
    }

    // Canonical trace id comes from the earliest-starting span's worker.
    let canonical_trace_id = all_spans
        .iter()
        .min_by_key(|s| s.start_time_unix_nano)
        .map(|s| s.trace_id.clone())
        .expect("non-empty span list");

    for span in &mut all_spans {
        span.trace_id = canonical_trace_id.clone();
    }

    let mut known_span_ids: HashSet<String> = all_spans.iter().map(|s| s.span_id.clone()).collect();

    let earliest_root = all_spans
        .iter()
        .filter(|s| s.parent_span_id.is_none())
        .min_by_key(|s| s.start_time_unix_nano)
        .cloned();

    let synthetic_id = "0".repeat(16);
    let (synthesized_root, attach_parent_id) = match earliest_root {
        Some(root) => (false, root.span_id),
        None => {
            let orphan_min_start = all_spans
                .iter()
                .map(|s| s.start_time_unix_nano)
                .min()
                .unwrap_or_default();
            let end_max = all_spans
                .iter()
                .map(|s| s.end_time_unix_nano)
                .max()
                .unwrap_or(orphan_min_start);

            known_span_ids.insert(synthetic_id.clone());
            all_spans.push(OtelSpan {
                trace_id: canonical_trace_id.clone(),
                span_id: synthetic_id.clone(),
                parent_span_id: None,
                name: "build.aggregated".to_string(),
                kind: crate::otel::SpanKind::Server,
                start_time_unix_nano: orphan_min_start,
                end_time_unix_nano: end_max,
                attributes: HashMap::from([(
                    "fish.aggregated".to_string(),
                    crate::otel::AttributeValue::Bool(true),
                )]),
                events: Vec::new(),
                status: crate::otel::SpanStatus {
                    code: crate::otel::StatusCode::Ok,
                    message: None,
                },
            });
            (true, synthetic_id.clone())
        }
    };

    let mut re_parented_orphans = 0usize;
    for span in &mut all_spans {
        let orphan = match &span.parent_span_id {
            Some(parent) => !known_span_ids.contains(parent.as_str()),
            None => false,
        };
        if orphan && span.span_id != attach_parent_id {
            span.parent_span_id = Some(attach_parent_id.clone());
            re_parented_orphans += 1;
        }
    }

    let stats = MergeStats {
        source_workers: tracers.len(),
        total_spans: all_spans.len(),
        dropped_duplicates,
        re_parented_orphans,
        synthesized_root,
    };

    let merged_tracer = OtelTracer::with_trace_id("fish-build", canonical_trace_id);
    merged_tracer.record_spans(all_spans);
    (merged_tracer, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otel::OtelTracer;

    fn make_tracer(service: &str) -> OtelTracer {
        OtelTracer::with_trace_id(service, format!("{service:0<32}"))
    }

    #[test]
    fn test_merge_deduplicates_exact_spans() {
        let a = make_tracer("aaaa");
        let b = make_tracer("bbbb");

        let span_a = a.start_span("compile").finish(true, None);
        b.record_span(span_a.clone());
        b.record_span(span_a); // duplicate arrives from second exporter

        let (merged, stats) = merge_worker_traces(&[&a, &b]);
        assert_eq!(stats.total_spans, 1);
        assert_eq!(stats.dropped_duplicates, 1);
        assert_eq!(merged.span_count(), 1);
    }

    #[test]
    fn test_merge_adopts_earliest_worker_trace_id() {
        let slow = make_tracer("1111");
        let fast = OtelTracer::with_trace_id("fast", "22222222222222222222222222222222");

        // fast worker's span starts first and must win the canonical id
        let early = fast.start_span("early").finish(true, None);
        fast.record_span(early);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let mut late = slow.start_span("late");
        late.add_event("marker", HashMap::new());
        slow.record_span(late.finish(true, None));

        let (merged, _stats) = merge_worker_traces(&[&slow, &fast]);
        for span in merged.spans() {
            assert_eq!(span.trace_id, "22222222222222222222222222222222");
        }
    }

    #[test]
    fn test_merge_reparents_orphans_onto_earliest_root() {
        let root_owner = make_tracer("root");
        let orphan_owner = make_tracer("orph");

        let root = root_owner.start_span("build").finish(true, None);
        root_owner.record_span(root);

        // orphan references a parent id that never arrived
        let orphan = orphan_owner
            .start_span("remote_task")
            .with_parent("deadbeef");
        orphan_owner.record_span(orphan.finish(true, None));

        let (merged, stats) = merge_worker_traces(&[&root_owner, &orphan_owner]);
        assert_eq!(stats.re_parented_orphans, 1);
        assert!(!stats.synthesized_root);

        let spans = merged.spans();
        let reparented = spans
            .iter()
            .find(|s| s.name == "remote_task")
            .expect("orphan must survive");
        let root_id = spans
            .iter()
            .find(|s| s.name == "build")
            .unwrap()
            .span_id
            .clone();
        assert_eq!(reparented.parent_span_id.as_deref(), Some(root_id.as_str()));
    }

    #[test]
    fn test_merge_synthesizes_root_when_none_exists() {
        let w1 = make_tracer("w1");
        let w2 = make_tracer("w2");

        let t1 = w1.start_span("task_one").with_parent("missing");
        w1.record_span(t1.finish(true, None));

        let t2 = w2.start_span("task_two").with_parent("also-missing");
        w2.record_span(t2.finish(true, None));

        let (merged, stats) = merge_worker_traces(&[&w1, &w2]);
        assert!(stats.synthesized_root);
        assert_eq!(stats.total_spans, 3);

        let spans = merged.spans();
        let synthetic = spans.iter().find(|s| s.name == "build.aggregated").unwrap();
        for span in &spans {
            if span.name == "build.aggregated" {
                continue;
            }
            assert_eq!(
                span.parent_span_id.as_deref(),
                Some(synthetic.span_id.as_str())
            );
        }
    }

    #[test]
    fn test_merge_empty_inputs_stay_empty() {
        let empty = make_tracer("empty");
        let (merged, stats) = merge_worker_traces(&[&empty]);
        assert_eq!(stats.total_spans, 0);
        assert!(!stats.synthesized_root);
        assert_eq!(merged.span_count(), 0);
    }
}

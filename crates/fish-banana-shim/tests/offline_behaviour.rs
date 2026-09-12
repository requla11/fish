//! Integration tests for the offline `banana` shim.
//!
//! These pin down the behaviour Fish relies on while the real
//! `submodules/banana` implementation is unavailable (private repo):
//! symbol extraction, energy estimation, OCI packing, the P2P map, and the
//! Merkle ledger.

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ast
// ---------------------------------------------------------------------------

#[test]
fn extract_symbols_finds_rust_functions_and_structs() {
    let src = "pub fn run_build() {}\nfn helper(x: u8) -> u8 { x }\npub struct BuildConfig;\nstruct Hidden;\n";
    let symbols = banana::ast::PolyglotAstEngine::extract_symbols(src, "rs");
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"run_build"), "names: {names:?}");
    assert!(names.contains(&"helper"), "names: {names:?}");
    assert!(names.contains(&"BuildConfig"), "names: {names:?}");
    assert!(names.contains(&"Hidden"), "names: {names:?}");
    assert!(symbols.iter().all(|s| !s.kind.is_empty()));
}

#[test]
fn extract_symbols_finds_python_defs_and_classes() {
    let src = "def build(target):\n    pass\n\nclass Builder:\n    pass\n";
    let symbols = banana::ast::PolyglotAstEngine::extract_symbols(src, "py");
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"build"), "names: {names:?}");
    assert!(names.contains(&"Builder"), "names: {names:?}");
}

#[test]
fn extract_symbols_finds_generic_functions_and_classes() {
    let src = "function buildAll() {}\nclass BuildRunner {}\n";
    let symbols = banana::ast::PolyglotAstEngine::extract_symbols(src, "ts");
    assert_eq!(symbols.len(), 2, "symbols: {symbols:?}");
    assert_eq!(symbols.first().unwrap().name, "buildAll");
    assert_eq!(symbols.first().unwrap().kind, "function");
    assert_eq!(symbols.get(1).unwrap().name, "BuildRunner");
    assert_eq!(symbols.get(1).unwrap().kind, "class");
}

#[test]
fn extract_symbols_ignores_non_definition_lines() {
    let src = "let x = 1;\nprintln!(\"hello\");\nreturn x;\n";
    assert!(banana::ast::PolyglotAstEngine::extract_symbols(src, "rs").is_empty());
}

#[test]
fn extract_symbols_scans_line_by_line_without_comment_awareness() {
    // Documented shim limitation: definition keywords are matched anywhere
    // in a line, so commented-out definitions are still extracted. The real
    // `banana` engine replaces this with full AST parsing.
    let src = "// fn commented_out() {}\n";
    let symbols = banana::ast::PolyglotAstEngine::extract_symbols(src, "rs");
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols.first().unwrap().name, "commented_out");
}

#[test]
fn dependency_graph_reports_no_cycles_in_shim() {
    let mut graph = banana::ast::DependencyGraph::new();
    graph.nodes.insert(PathBuf::from("a.rs"));
    graph.nodes.insert(PathBuf::from("b.rs"));
    assert!(graph.detect_cycles().is_empty());
}

#[test]
fn semantic_symbol_serializes_round_trip() {
    let sym = banana::ast::SemanticSymbol::new("main", "function");
    let json = serde_json::to_string(&sym).unwrap();
    let back: banana::ast::SemanticSymbol = serde_json::from_str(&json).unwrap();
    assert_eq!(back, sym);
}

// ---------------------------------------------------------------------------
// telemetry
// ---------------------------------------------------------------------------

const fn hw_profile() -> banana::telemetry::HardwareProfile {
    banana::telemetry::HardwareProfile {
        tdp_watts: 100.0,
        idle_power_watts: 10.0,
        core_count: 8,
    }
}

#[test]
fn energy_meter_estimates_scale_with_utilization() {
    let mut low = banana::telemetry::EnergyMeter::new(hw_profile(), 400.0);
    low.start();
    std::thread::sleep(std::time::Duration::from_millis(30));
    let low_metrics = low.stop(0.25);

    let mut high = banana::telemetry::EnergyMeter::new(hw_profile(), 400.0);
    high.start();
    std::thread::sleep(std::time::Duration::from_millis(30));
    let high_metrics = high.stop(1.0);

    assert!(high_metrics.estimated_joules > low_metrics.estimated_joules);
    assert!(high_metrics.carbon_grams_co2 > low_metrics.carbon_grams_co2);
    assert!(high_metrics.energy_wh > low_metrics.energy_wh);
    assert!(high_metrics.cpu_cores_utilized > low_metrics.cpu_cores_utilized);
}

#[test]
fn energy_meter_clamps_utilization_into_unit_range() {
    let mut over_meter = banana::telemetry::EnergyMeter::new(hw_profile(), 400.0);
    over_meter.start();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let over = over_meter.stop(5.0);

    let mut under_meter = banana::telemetry::EnergyMeter::new(hw_profile(), 400.0);
    under_meter.start();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let under = under_meter.stop(-1.0);

    assert!((over.cpu_cores_utilized - 8.0).abs() < f64::EPSILON);
    assert!((under.cpu_cores_utilized - f64::EPSILON).abs() < f64::EPSILON);
}

#[test]
fn energy_meter_fresh_session_estimates_zero() {
    // stop() without start() measures a zero-length session.
    let meter = banana::telemetry::EnergyMeter::new(hw_profile(), 400.0);
    let metrics = meter.stop(1.0);
    assert!(metrics.estimated_joules.abs() < f64::EPSILON);
    assert!(metrics.energy_wh.abs() < f64::EPSILON);
    assert!(metrics.carbon_grams_co2.abs() < f64::EPSILON);
}

#[test]
fn energy_metrics_deserializes_legacy_payload() {
    let legacy =
        r#"{"cpu_cores_utilized": 4.0, "estimated_joules": 120.0, "carbon_grams_co2": 9.0}"#;
    let m: banana::telemetry::EnergyMetrics = serde_json::from_str(legacy).unwrap();
    assert!((m.estimated_joules - 120.0).abs() < f64::EPSILON);
    // Derived/compat fields default to zero when absent.
    assert!(m.energy_wh.abs() < f64::EPSILON);
    assert!(m.co2_g.abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// oci
// ---------------------------------------------------------------------------

#[test]
fn oci_builder_packs_rootfs_and_reports_digest() {
    let temp = tempfile::tempdir().unwrap();
    let rootfs = temp.path().join("rootfs");
    std::fs::create_dir_all(rootfs.join("bin")).unwrap();
    std::fs::write(rootfs.join("bin/hello"), b"hello").unwrap();

    let out_tar = temp.path().join("image.tar");
    let builder = banana::oci::OciBuilder::new()
        .entrypoint(vec!["/bin/hello".to_string()])
        .working_dir("/work");
    let result = builder.build_from_rootfs(&rootfs, &out_tar).unwrap();

    assert!(result.manifest_digest.starts_with("sha256:"));
    assert_eq!(result.manifest_digest.len(), "sha256:".len() + 64);
    assert!(result.bytes_written > 0);
    assert!(out_tar.exists());
}

#[test]
fn oci_builder_fails_on_missing_rootfs() {
    let temp = tempfile::tempdir().unwrap();
    let builder = banana::oci::OciBuilder::new();
    let err = builder
        .build_from_rootfs(&temp.path().join("missing"), &temp.path().join("out.tar"))
        .unwrap_err();
    assert!(
        err.to_string().contains("rootfs does not exist"),
        "err: {err}"
    );
}

// ---------------------------------------------------------------------------
// p2p
// ---------------------------------------------------------------------------

#[test]
fn p2p_node_stores_and_fetches_artifacts() {
    let node = banana::p2p::P2PNode::new("node-1", "127.0.0.1:7000".parse().unwrap());
    assert_eq!(node.node_id(), "node-1");
    assert!(node.get_artifact("missing").is_none());

    node.store_artifact("key", vec![1, 2, 3]);
    assert_eq!(node.get_artifact("key"), Some(vec![1, 2, 3]));

    // Re-storing overwrites the previous value.
    node.store_artifact("key", vec![9]);
    assert_eq!(node.get_artifact("key"), Some(vec![9]));
}

#[test]
fn p2p_swarm_manager_registers_unique_peers() {
    let node = banana::p2p::P2PNode::new("local", "127.0.0.1:7001".parse().unwrap());
    let swarm = banana::p2p::P2PSwarmManager::new(node);
    assert_eq!(swarm.peer_count(), 0);

    let peer = banana::p2p::PeerDescriptor {
        node_id: "peer-a".to_string(),
        addr: "127.0.0.1:7002".parse().unwrap(),
    };
    swarm.register_peer(peer.clone());
    swarm.register_peer(peer);
    assert_eq!(swarm.peer_count(), 1);

    // The shim has no distributed index: lookups always come back empty.
    assert!(swarm.find_peers_with_artifact("anything").is_empty());
}

// ---------------------------------------------------------------------------
// ledger
// ---------------------------------------------------------------------------

#[test]
fn ledger_appends_records_with_sequence_numbers() {
    let mut ledger = banana::ledger::LedgerWitness::new();
    assert_eq!(ledger.append_record("a", "hash-a", "builder"), 0);
    assert_eq!(ledger.append_record("b", "hash-b", "builder"), 1);
    assert_eq!(ledger.records().len(), 2);
    assert_eq!(ledger.records().first().unwrap().seq, 0);
    assert_eq!(ledger.records().get(1).unwrap().seq, 1);
}

#[test]
fn ledger_record_maps_to_slsa_statement() {
    let record = banana::ledger::LedgerRecord {
        seq: 0,
        artifact_name: "app".to_string(),
        artifact_hash: "deadbeef".to_string(),
        builder_id: "fish".to_string(),
    };
    let stmt = record.to_slsa_v1_statement();
    assert_eq!(stmt.statement_type, "https://in-toto.io/Statement/v1");
    assert_eq!(stmt.subject.len(), 1);
    let subject = stmt.subject.first().unwrap();
    assert_eq!(subject.name, "app");
    assert_eq!(
        subject.digest.get("sha256").map(String::as_str),
        Some("deadbeef")
    );
    assert_eq!(stmt.predicate.builder_id, "fish");
    assert_eq!(stmt.predicate.build_type, "https://slsa.dev/provenance/v1");
}

#[test]
fn ledger_persists_json_lines() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("nested").join("ledger.jsonl");

    let mut ledger = banana::ledger::LedgerWitness::new();
    ledger.append_record("a", "hash-a", "builder");
    ledger.append_record("b", "hash-b", "builder");
    ledger.persist_to_disk(&path).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    let back: banana::ledger::LedgerRecord = serde_json::from_str(lines.first().unwrap()).unwrap();
    assert_eq!(back.artifact_name, "a");
    assert_eq!(back.seq, 0);
}

#[test]
fn merkle_tree_roots_are_deterministic_and_content_bound() {
    let leaf_a = *blake3::hash(b"a").as_bytes();
    let leaf_b = *blake3::hash(b"b").as_bytes();

    let empty = banana::ledger::MerkleTree::from_leaves(Vec::new());
    let single = banana::ledger::MerkleTree::from_leaves(vec![leaf_a]);
    let pair = banana::ledger::MerkleTree::from_leaves(vec![leaf_a, leaf_b]);
    let pair_again = banana::ledger::MerkleTree::from_leaves(vec![leaf_a, leaf_b]);
    let swapped = banana::ledger::MerkleTree::from_leaves(vec![leaf_b, leaf_a]);

    // Deterministic for identical input.
    assert_eq!(pair.root_hash(), pair_again.root_hash());
    // Sensitive to leaf ordering and content.
    assert_ne!(pair.root_hash(), swapped.root_hash());
    assert_ne!(single.root_hash(), empty.root_hash());
    // Root hashes are 64 hex characters.
    assert_eq!(empty.root_hash().len(), 64);
    assert!(empty.root_hash().chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn ledger_signs_tree_root_deterministically() {
    let mut ledger = banana::ledger::LedgerWitness::new();
    ledger.append_record("a", "hash-a", "builder");
    let tree = ledger.build_tree();

    let (sig, sig_bytes) = ledger.sign_root(&tree);
    assert_eq!(sig, format!("sig:{}", tree.root_hash()));
    assert_eq!(sig_bytes, sig.as_bytes());

    let (sig_again, _) = ledger.sign_root(&tree);
    assert_eq!(sig, sig_again);
}

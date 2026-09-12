//! Offline fallback for the private `banana` submodule.
//!
//! The real `submodules/banana` repo (P2P swarm, OCI builder, AST engine,
//! ledger, telemetry) is currently private. This shim implements just enough
//! of the API used by Fish (`ast`, `telemetry`, `oci`, `p2p`, `ledger`) so
//! the workspace builds and unit tests pass offline.
//!
//! To restore the full implementation once the repos are public:
//! ```sh
//! rm -rf submodules/banana
//! git submodule update --init --recursive
//! # then point [workspace.dependencies] banana back to submodules/banana
//! ```

#![forbid(unsafe_code)]
#![allow(missing_docs)]

pub mod ast {
    use serde::{Deserialize, Serialize};
    use std::collections::HashSet;
    use std::path::PathBuf;

    /// A symbol extracted from source code.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct SemanticSymbol {
        /// Symbol name (function, struct, class, ...).
        pub name: String,
        /// Symbol kind, e.g. "function", "struct".
        #[serde(default)]
        pub kind: String,
    }

    impl SemanticSymbol {
        /// Create a new symbol.
        #[must_use]
        pub fn new(name: impl Into<String>, kind: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                kind: kind.into(),
            }
        }
    }

    /// Minimal dependency graph used for cycle checks.
    #[derive(Debug, Clone, Default)]
    pub struct DependencyGraph {
        /// Known nodes. Edges are not modelled in the shim (no cycles).
        pub nodes: HashSet<PathBuf>,
    }

    impl DependencyGraph {
        /// Create an empty graph.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Detect cycles. The shim tracks nodes only, so it never reports any.
        #[must_use]
        pub const fn detect_cycles(&self) -> Vec<Vec<PathBuf>> {
            Vec::new()
        }
    }

    /// Best-effort multi-language symbol extractor.
    pub struct PolyglotAstEngine;

    impl PolyglotAstEngine {
        /// Extract `fn`/`struct`/`class`/`def` symbols from `content`.
        ///
        /// This is intentionally simple: it scans line-by-line for common
        /// definition keywords so offline builds still get dependency edges.
        /// The real `banana` engine uses full AST parsing.
        #[must_use]
        pub fn extract_symbols(content: &str, _ext: &str) -> Vec<SemanticSymbol> {
            let mut out = Vec::new();
            for line in content.lines() {
                let line = line.trim();
                // Rust: `pub fn name(`, `fn name(`, `pub struct Name`, `struct Name`
                if let Some(sym) = parse_rust_fn(line).or_else(|| parse_rust_struct(line)) {
                    out.push(sym);
                    continue;
                }
                // Python: `def name(`
                if let Some(sym) = parse_python_def(line) {
                    out.push(sym);
                    continue;
                }
                // TS/JS/Go: `function name(`, `class Name`
                if let Some(sym) = parse_generic_fn_or_class(line) {
                    out.push(sym);
                }
            }
            out
        }
    }

    fn ident_after(s: &str) -> Option<String> {
        let s = s.trim_start_matches(['_', '*']);
        let end = s
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(s.len());
        let ident = &s[..end];
        if ident.is_empty() {
            None
        } else {
            Some(ident.to_string())
        }
    }

    fn parse_rust_fn(line: &str) -> Option<SemanticSymbol> {
        let idx = line.find("fn ")?;
        let after = line[idx + 3..].trim_start();
        // Skip closures / fn pointers without a name.
        let name = ident_after(after)?;
        Some(SemanticSymbol::new(name, "function"))
    }

    fn parse_rust_struct(line: &str) -> Option<SemanticSymbol> {
        let idx = line.find("struct ")?;
        let after = line[idx + 7..].trim_start();
        let name = ident_after(after)?;
        Some(SemanticSymbol::new(name, "struct"))
    }

    fn parse_python_def(line: &str) -> Option<SemanticSymbol> {
        let stripped = line.strip_prefix("def ")?.trim_start();
        let name = ident_after(stripped)?;
        Some(SemanticSymbol::new(name, "function"))
    }

    fn parse_generic_fn_or_class(line: &str) -> Option<SemanticSymbol> {
        if let Some(idx) = line.find("function ") {
            let name = ident_after(line[idx + 9..].trim_start())?;
            return Some(SemanticSymbol::new(name, "function"));
        }
        if let Some(idx) = line.find("class ") {
            let name = ident_after(line[idx + 6..].trim_start())?;
            return Some(SemanticSymbol::new(name, "class"));
        }
        None
    }
}

pub mod telemetry {
    use serde::{Deserialize, Serialize};
    use std::time::Instant;

    /// Hardware power profile.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HardwareProfile {
        /// Thermal design power in watts.
        pub tdp_watts: f64,
        /// Idle power draw in watts.
        pub idle_power_watts: f64,
        /// Number of CPU cores.
        pub core_count: usize,
    }

    /// Energy measurement for one build session.
    ///
    /// Field names match the real `banana::telemetry` API as used by
    /// `fish-cli/src/build.rs` (`estimated_joules` / `carbon_grams_co2`).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EnergyMetrics {
        /// Effective cores utilized (utilization * `core_count`).
        pub cpu_cores_utilized: f64,
        /// Estimated energy in joules.
        pub estimated_joules: f64,
        /// Estimated CO2 in grams.
        pub carbon_grams_co2: f64,
        /// Estimated energy in watt-hours (derived, kept for compat).
        #[serde(default)]
        pub energy_wh: f64,
        /// Estimated CO2 in grams (alias, kept for compat).
        #[serde(default)]
        pub co2_g: f64,
    }

    /// Simple wall-clock energy estimator.
    #[derive(Debug, Clone)]
    pub struct EnergyMeter {
        profile: HardwareProfile,
        grid_intensity_g_per_kwh: f64,
        start: Option<Instant>,
    }

    impl EnergyMeter {
        /// Create a meter for the given hardware and grid intensity.
        #[must_use]
        pub const fn new(profile: HardwareProfile, grid_intensity_g_per_kwh: f64) -> Self {
            Self {
                profile,
                grid_intensity_g_per_kwh,
                start: None,
            }
        }

        /// Start a measurement session.
        pub fn start(&mut self) {
            self.start = Some(Instant::now());
        }

        /// Stop and estimate energy for `cpu_utilization` in `[0, 1]`.
        #[must_use]
        pub fn stop(&self, cpu_utilization: f64) -> EnergyMetrics {
            let elapsed_secs = self.start.map_or(0.0, |s| s.elapsed().as_secs_f64());
            let util = cpu_utilization.clamp(0.0, 1.0);
            // `u32::MAX` cores is far beyond any real machine, so saturating
            // there keeps the `u32 -> f64` conversion below an audit without
            // changing results on real hardware.
            let cores_u32 = u32::try_from(self.profile.core_count.max(1)).unwrap_or(u32::MAX);
            let cores = f64::from(cores_u32);
            let dynamic_w = self.profile.tdp_watts - self.profile.idle_power_watts;
            let power_w = dynamic_w.mul_add(util, self.profile.idle_power_watts);
            let energy_wh = power_w * elapsed_secs / 3600.0;
            let estimated_joules = power_w * elapsed_secs;
            let carbon_grams_co2 = energy_wh / 1000.0 * self.grid_intensity_g_per_kwh;
            EnergyMetrics {
                cpu_cores_utilized: (util * cores).max(f64::EPSILON),
                estimated_joules,
                carbon_grams_co2,
                energy_wh,
                co2_g: carbon_grams_co2,
            }
        }
    }
}

pub mod oci {
    use serde::{Deserialize, Serialize};
    use std::path::Path;

    /// Result of building an OCI image tarball.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OciImageResult {
        /// `sha256:<hex>` digest of the output tarball.
        pub manifest_digest: String,
        /// Number of bytes written.
        pub bytes_written: u64,
    }

    /// Minimal OCI image builder (rootfs -> tar).
    #[derive(Debug, Clone, Default)]
    pub struct OciBuilder {
        entrypoint: Vec<String>,
        working_dir: String,
    }

    impl OciBuilder {
        /// Create a builder with defaults.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Set the container entrypoint.
        #[must_use]
        pub fn entrypoint(mut self, entrypoint: Vec<String>) -> Self {
            self.entrypoint = entrypoint;
            self
        }

        /// Set the working directory.
        #[must_use]
        pub fn working_dir(mut self, working_dir: impl Into<String>) -> Self {
            self.working_dir = working_dir.into();
            self
        }

        /// Pack `rootfs` into `output_tar` and return its digest.
        ///
        /// # Errors
        ///
        /// Returns an error when `rootfs` does not exist, when the output
        /// tarball cannot be created or written, or when the digest cannot
        /// be computed.
        pub fn build_from_rootfs(
            &self,
            rootfs: &Path,
            output_tar: &Path,
        ) -> Result<OciImageResult, anyhow::Error> {
            if !rootfs.exists() {
                anyhow::bail!("rootfs does not exist: {}", rootfs.display());
            }
            if let Some(parent) = output_tar.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tar_file = std::fs::File::create(output_tar)?;
            let mut builder = tar::Builder::new(tar_file);
            builder.append_dir_all(".", rootfs)?;
            // Record entrypoint/workdir as a small metadata file so the
            // output is not just a verbatim copy.
            let meta = serde_json::json!({
                "entrypoint": self.entrypoint,
                "working_dir": self.working_dir,
            });
            let meta_bytes = serde_json::to_vec(&meta)?;
            let mut header = tar::Header::new_gnu();
            header.set_size(meta_bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, "fish-oci-config.json", &meta_bytes[..])?;
            builder.finish()?;

            let bytes = std::fs::read(output_tar)?;
            let digest = format!("sha256:{}", hex_sha256(&bytes));
            Ok(OciImageResult {
                manifest_digest: digest,
                bytes_written: bytes.len() as u64,
            })
        }
    }

    fn hex_sha256(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        use std::fmt::Write as _;
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let out = hasher.finalize();
        let mut s = String::with_capacity(out.len() * 2);
        for b in out {
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

pub mod p2p {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    /// A peer in the local swarm.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PeerDescriptor {
        /// Peer node id.
        pub node_id: String,
        /// Address the peer listens on.
        pub addr: SocketAddr,
    }

    /// Local node with an in-memory artifact map.
    #[derive(Debug, Clone)]
    pub struct P2PNode {
        node_id: String,
        bind_addr: SocketAddr,
        artifacts: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    impl P2PNode {
        /// Create a node.
        #[must_use]
        pub fn new(node_id: impl Into<String>, bind_addr: SocketAddr) -> Self {
            Self {
                node_id: node_id.into(),
                bind_addr,
                artifacts: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        /// Store an artifact locally.
        pub fn store_artifact(&self, key: &str, data: Vec<u8>) {
            if let Ok(mut map) = self.artifacts.lock() {
                map.insert(key.to_string(), data);
            }
        }

        /// Fetch a locally stored artifact.
        #[must_use]
        pub fn get_artifact(&self, key: &str) -> Option<Vec<u8>> {
            self.artifacts
                .lock()
                .ok()
                .and_then(|map| map.get(key).cloned())
        }

        /// Node identifier.
        #[must_use]
        pub fn node_id(&self) -> &str {
            &self.node_id
        }

        /// Bind address.
        #[must_use]
        pub const fn bind_addr(&self) -> SocketAddr {
            self.bind_addr
        }
    }

    /// Minimal swarm manager (local node + known peers).
    #[derive(Debug, Clone)]
    pub struct P2PSwarmManager {
        local: P2PNode,
        peers: Arc<Mutex<Vec<PeerDescriptor>>>,
    }

    impl P2PSwarmManager {
        /// Create a manager for `local`.
        #[must_use]
        pub fn new(local: P2PNode) -> Self {
            Self {
                local,
                peers: Arc::new(Mutex::new(Vec::new())),
            }
        }

        /// Access the local node.
        #[must_use]
        pub const fn get_local_node(&self) -> &P2PNode {
            &self.local
        }

        /// Register a peer.
        pub fn register_peer(&self, peer: PeerDescriptor) {
            if let Ok(mut peers) = self.peers.lock()
                && !peers.iter().any(|p| p.node_id == peer.node_id)
            {
                peers.push(peer);
            }
        }

        /// Find peers claiming `key`. The shim has no distributed index,
        /// so it returns an empty list (local hits go through the node).
        #[must_use]
        pub const fn find_peers_with_artifact(&self, _key: &str) -> Vec<PeerDescriptor> {
            Vec::new()
        }

        /// Number of known peers.
        #[must_use]
        pub fn peer_count(&self) -> usize {
            self.peers.lock().map_or(0, |p| p.len())
        }
    }
}

pub mod ledger {
    use serde::{Deserialize, Serialize};
    use std::path::Path;

    /// An in-toto style statement for one artifact.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InTotoStatement {
        /// Statement type URI.
        #[serde(rename = "_type")]
        pub statement_type: String,
        /// Subject (artifact name + digest).
        pub subject: Vec<StatementSubject>,
        /// Predicate (builder id, build type).
        pub predicate: StatementPredicate,
    }

    /// A single subject entry.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StatementSubject {
        /// Artifact name.
        pub name: String,
        /// Digest map, e.g. {"blake3": "feedbeef"}.
        pub digest: std::collections::HashMap<String, String>,
    }

    /// Predicate metadata.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StatementPredicate {
        /// Builder identifier.
        pub builder_id: String,
        /// Build type URI.
        pub build_type: String,
    }

    /// One ledger record.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LedgerRecord {
        /// Sequence number.
        pub seq: u64,
        /// Artifact name.
        pub artifact_name: String,
        /// Artifact hash.
        pub artifact_hash: String,
        /// Builder id.
        pub builder_id: String,
    }

    impl LedgerRecord {
        /// Render as an SLSA v1 statement.
        #[must_use]
        pub fn to_slsa_v1_statement(&self) -> InTotoStatement {
            let mut digest = std::collections::HashMap::new();
            digest.insert("sha256".to_string(), self.artifact_hash.clone());
            InTotoStatement {
                statement_type: "https://in-toto.io/Statement/v1".to_string(),
                subject: vec![StatementSubject {
                    name: self.artifact_name.clone(),
                    digest,
                }],
                predicate: StatementPredicate {
                    builder_id: self.builder_id.clone(),
                    build_type: "https://slsa.dev/provenance/v1".to_string(),
                },
            }
        }
    }

    /// Append-only in-memory ledger with Merkle hashing.
    #[derive(Debug, Clone, Default)]
    pub struct LedgerWitness {
        records: Vec<LedgerRecord>,
    }

    impl LedgerWitness {
        /// Create an empty ledger.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Append a record and return its sequence number.
        pub fn append_record(
            &mut self,
            artifact_name: impl Into<String>,
            artifact_hash: impl Into<String>,
            builder_id: impl Into<String>,
        ) -> u64 {
            let seq = self.records.len() as u64;
            self.records.push(LedgerRecord {
                seq,
                artifact_name: artifact_name.into(),
                artifact_hash: artifact_hash.into(),
                builder_id: builder_id.into(),
            });
            seq
        }

        /// Access all records.
        #[must_use]
        pub fn records(&self) -> &[LedgerRecord] {
            &self.records
        }

        /// Build a Merkle tree over the records.
        #[must_use]
        pub fn build_tree(&self) -> MerkleTree {
            let leaves: Vec<[u8; 32]> = self
                .records
                .iter()
                .map(|r| blake3::hash(serde_json::to_vec(r).unwrap_or_default().as_slice()))
                .map(|h| *h.as_bytes())
                .collect();
            MerkleTree::from_leaves(leaves)
        }

        /// Sign the tree root (placeholder hex signature).
        #[must_use]
        pub fn sign_root(&self, tree: &MerkleTree) -> (String, Vec<u8>) {
            let sig = format!("sig:{}", tree.root_hash());
            (sig.clone(), sig.into_bytes())
        }

        /// Persist the ledger as JSON lines.
        ///
        /// # Errors
        ///
        /// Returns an error when the parent directory cannot be created,
        /// when a record cannot be serialized, or when the file cannot
        /// be written.
        pub fn persist_to_disk(&self, path: &Path) -> Result<(), anyhow::Error> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = String::new();
            for r in &self.records {
                out.push_str(&serde_json::to_string(r)?);
                out.push('\n');
            }
            std::fs::write(path, out)?;
            Ok(())
        }
    }

    /// Minimal binary Merkle tree.
    #[derive(Debug, Clone)]
    pub struct MerkleTree {
        root: [u8; 32],
    }

    impl MerkleTree {
        /// Build from leaf hashes (empty tree hashes empty input).
        #[must_use]
        pub fn from_leaves(mut leaves: Vec<[u8; 32]>) -> Self {
            if leaves.is_empty() {
                return Self {
                    root: *blake3::hash(b"fish-banana-shim:empty").as_bytes(),
                };
            }
            while leaves.len() > 1 {
                let mut next = Vec::with_capacity(leaves.len().div_ceil(2));
                for pair in leaves.chunks(2) {
                    if let [a, b] = pair {
                        let mut h = blake3::Hasher::new();
                        h.update(a);
                        h.update(b);
                        next.push(*h.finalize().as_bytes());
                    } else if let [single] = pair {
                        next.push(*single);
                    }
                }
                leaves = next;
            }
            Self {
                root: leaves.into_iter().next().unwrap_or([0u8; 32]),
            }
        }

        /// Hex-encoded root hash (never empty).
        #[must_use]
        pub fn root_hash(&self) -> String {
            use std::fmt::Write as _;
            let mut s = String::with_capacity(64);
            for b in self.root {
                let _ = write!(s, "{b:02x}");
            }
            s
        }
    }
}

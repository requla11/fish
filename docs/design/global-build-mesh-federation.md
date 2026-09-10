# Design Document: Global Build Mesh Federation (Moonshot)

## 1. Overview
The **Global Build Mesh Federation** is a v2.0 Moonshot initiative aiming to radically improve cold-cache hit rates by allowing independent organizations to share anonymized CAS (Content Addressable Storage) chunks peer-to-peer. 

Unlike the existing `BuildGrid` (which federates compute and cache *within* a single organization's trust boundary), the Global Build Mesh spans zero-trust boundaries. This enables the open-source community and willing enterprises to collectively cache widely used, deterministic dependency graphs (e.g., compiling `tokio v1.40.0` or `react v18`).

## 2. Goals & Non-Goals

### 2.1 Goals
- **Opt-in sharing**: Organizations can voluntarily publish deterministic artifacts to the public mesh.
- **Zero-trust verification**: Any fetched artifact must be cryptographically verified via BLAKE3 before execution.
- **Privacy by default**: Only public dependencies (e.g., crates.io, npm) are shared. Internal proprietary code is strictly isolated.
- **Decentralized topology**: BitTorrent-inspired DHT (Distributed Hash Table) for locating CAS chunks.

### 2.2 Non-Goals
- Sharing compiled application code or proprietary artifacts.
- Modifying the existing internal-only `fish-remote-cache` mesh (this is a separate layer).
- Replacing standard package managers.

## 3. Architecture

### 3.1 Topology & DHT
The Global Mesh operates as a Kademlia-based DHT.
When `fish build` encounters a cold cache for a public package:
1. It queries the DHT for the BLAKE3 hash of the required compilation output.
2. If peers are found, it streams the ZSTD-compressed CAS chunk directly from them via WebRTC or QUIC.
3. Upon receiving the chunk, the local `fish-cas` strictly verifies the hash. If the hash matches, the chunk is trusted (thanks to collision resistance).

### 3.2 Proof of Work / Quotas
To prevent freeloading and Sybil attacks, the mesh could implement a lightweight Web3-free reputation system or rate-limit based on a tit-for-tat protocol (similar to BitTorrent): you can only download at high speeds if you also seed artifacts.

### 3.3 Opt-in Configuration
In `fish.toml`:
```toml
[mesh.global]
enabled = true
seed_public_deps = true
bandwidth_limit = "50MB/s"
```

## 4. Security Considerations
- **Poisoning Attacks**: A malicious node might serve a backdoor binary with the correct hash? No, BLAKE3 ensures that if the content changes, the hash changes. The hash is derived from the *inputs* (source code, compiler version). Wait, if the hash is derived from inputs, how do we trust the mapping from `Hash(Inputs) -> Hash(Output)`?
- **The Mapping Problem**: We must rely on SLSA Level 3 attestations. A node providing a cached output must also provide a cryptographic proof (In-toto statement) signed by a trusted builder (e.g., GitHub Actions) linking the input hash to the output hash.

## 5. Prototype Implementation Plan
1. **Phase 1 (DHT Discovery)**: Introduce `libp2p` to `fish-remote-cache` for discovering peers across the internet.
2. **Phase 2 (SLSA Verification)**: Wire the `fish-security` SLSA verifier into the mesh download pipeline. Only accept chunks signed by trusted public CI keys (e.g., Fish Official Builders).
3. **Phase 3 (Opt-in Daemon)**: Extend `fish daemon` to seed verified chunks in the background.

## 6. Conclusion
This moonshot requires a robust trust model for the `Input -> Output` mapping, which our new SLSA Level 3 compliance engine perfectly solves. This feature will make `fish` the fastest build system on earth for open-source ecosystems.

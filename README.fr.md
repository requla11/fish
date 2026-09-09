<div align="center">

<img src="docs/public/logo.png" alt="Fish Logo" width="180" />

# 🐟 Fish

**Le Système d'Orchestration de Build Ultra-Rapide et Orienté Cache pour les Monorepos Polyglottes**

[![CI](https://github.com/requla11/fish/actions/workflows/dogfood.yaml/badge.svg)](https://github.com/requla11/fish/actions/workflows/dogfood.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/requla11/fish)

[English](README.md) • [Tiếng Việt](README.vi.md) • [简体中文](README.zh-hans.md) • [繁體中文](README.zh-hant.md) • [日本語](README.ja.md)

</div>

---

**Fish** est un moteur d'orchestration de build haute performance conçu en **Rust 2024**. Il offre la vitesse et la simplicité de Turborepo avec la puissance polyglotte de Bazel — **sans nécessiter de langages de configuration complexes comme Starlark ou de DSL de build personnalisés**.

Fish découvre automatiquement vos toolchains, analyse les arborescences sources pour déduire les dépendances (edges) inter-langages, planifie les tâches sur un pool work-stealing sans verrouillage (lock-free), et met en cache chaque artefact en utilisant un stockage adressable par contenu (CAS) cryptographiquement sécurisé basé sur **BLAKE3** et la compression **Zstandard**.

> 💡 **Remarque :** Fish coordonne les compilateurs et les gestionnaires de paquets existants (Cargo, Go, npm/pnpm, Python, Clang, etc.). Il ne les remplace pas. Il n'a aucun lien avec [fish-shell](https://fishshell.com) — ils ne partagent que le nom.

---

## ✨ Points Clés

| Fonctionnalité | Description |
| :--- | :--- |
| ⚡ **Planification Sous-Milliseconde (Sub-Millisecond Scheduling)** | Les files d'attente work-stealing Chase-Lev et la planification par chemin critique répartissent les tâches en <100µs. |
| 🌐 **Écosystème de 11+ Langages** | Backends natifs pour Rust, Go, TypeScript/JS, Python, C/C++, Java, .NET, Swift, Dart, Zig et Docker. |
| 🔗 **Inférence Automatique des Dépendances** | Liaison inter-langages "contract-first" : les références (comme `include_str!`, imports JSON) créent automatiquement des liens dans le DAG sans `depends_on` manuel. |
| 💾 **Cache CAS à Haut Débit** | Stockage adressable par contenu BLAKE3 dédupliqué avec mise en cache hiérarchisée L1/L2 et compression ZSTD. |
| 📡 **Cache P2P Zero-Config** | Partagez les artefacts de build de pair-à-pair sur le Wi-Fi / LAN local avec vos coéquipiers — sans frais de serveurs cloud. |
| 🛡️ **Isolation Hermétique** | Sandboxing multi-plateformes : namespaces Linux & Landlock, seatbelt macOS, et tokens de sécurité Windows. |
| 📊 **Interface Utilisateur Interactive en Temps Réel** | Dashboard web intégré (`fish ui`) comprenant un visualiseur interactif de DAG en SVG et des graphes de télémétrie. |

---

## 🚀 Installation Rapide

### Installateur en 1 ligne

#### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/requla11/fish/main/scripts/install.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/requla11/fish/main/scripts/install.ps1 | iex
```

---

### Gestionnaires de paquets

| Plateforme | Gestionnaire de paquets | Commande |
| :--- | :--- | :--- |
| **Windows** | **Scoop** | `scoop install https://raw.githubusercontent.com/requla11/fish/main/packaging/fish.json` |
| **Windows** | **Winget** | `winget install requla11.fish` |
| **macOS** | **Homebrew** | `brew tap requla11/fish https://github.com/requla11/homebrew-fish && brew install fish` |
| **Cargo** | **crates.io / Git** | `cargo install --git https://github.com/requla11/fish.git fish-cli` |

---

## 🏁 Démarrage Rapide

Naviguez vers n'importe quel dépôt multi-langages et exécutez :

```bash
# Compilez l'ensemble du workspace en parallèle avec une mise en cache intelligente
fish build

# Exécutez toutes les suites de tests pour chaque langage
fish test

# Mode watch : re-compile et re-teste lors de modifications de fichiers
fish dev

# Nettoyez les artefacts de build (ou nettoyez tout y compris le cache local avec --all)
fish clean --all

# Lancez le dashboard web interactif & le visualiseur de DAG
fish ui --open
```

### Essayez la Démo Polyglotte

Nous incluons un monorepo réaliste de type "contract-first" combinant **Rust + Go + Python + TypeScript** :

```bash
cd examples/polyglot-demo
fish build
fish graph --format tree
```

Output:
```text
🔗 Inferring cross-language dependencies:
   ↳ go-service → py-worker (Go project references `../py-worker/contracts/events.schema.json`)
   ↳ rust-service → py-worker (Rust project references `../../py-worker/contracts/events.schema.json`)
   ↳ web-frontend → py-worker (TypeScript project references `../../py-worker/contracts/topics.json`)
🔗 Linked 6 cross-project task edge(s) from 3 inference(s)

Build completed successfully.
  Tasks:     7 total (7 cached, 100% cache hit)
  Duration:  0.01s
```

---

## 🛠️ Écosystèmes Supportés

Fish détecte et orchestre nativement des projets à travers 11 écosystèmes majeurs :

| Écosystème | Manifeste Détecté | Tâches par Défaut |
| :--- | :--- | :--- |
| **Rust** | `Cargo.toml` | `cargo check`, `cargo build`, `cargo test` |
| **TypeScript / Node** | `package.json`, `tsconfig.json` | `typecheck`, `build`, `test` |
| **Go** | `go.mod` | `go vet`, `go build`, `go test` |
| **Python** | `pyproject.toml`, `requirements.txt` | compilation de la syntaxe, `pytest`, lint |
| **C / C++** | `CMakeLists.txt`, `fish.cc.json` | configuration CMake, build, `ctest` |
| **Java** | `pom.xml`, `build.gradle` | compile, test |
| **.NET / C#** | `*.csproj`, `*.sln` | `dotnet build`, `dotnet test` |
| **Swift** | `Package.swift` | `swift build`, `swift test` |
| **Dart / Flutter** | `pubspec.yaml` | `dart analyze`, `dart test` |
| **Zig** | `build.zig` | `zig build`, `zig test` |
| **Docker / OCI** | `Dockerfile`, `docker-compose.yml` | Build d'image multi-stage, compilation OCI |

---

## 📋 Commandes Essentielles

Fish garde son CLI clair, intuitif et facile à utiliser pour les développeurs :

```text
Build & Test:
  fish build             Compile toutes les cibles découvertes à partir du graphe de projet
  fish check             Vérifie les types et valide les cibles sans lier
  fish test              Exécute toutes les suites de tests à travers le workspace
  fish run [TARGET]      Compile et exécute une cible binaire spécifique
  fish dev (ou watch)    Observe en continu les fichiers et déclenche des rebuilds incrémentaux

Inspect & Understand:
  fish graph             Visualise le DAG sous forme d'arbres de phases, DOT ou JSON
  fish why <QUERY>       Demande en langage naturel pourquoi une cible a été reconstruite
  fish ui                Ouvre le dashboard web en temps réel & le visualiseur de DAG interactif
  fish doctor            Diagnostique les toolchains installées, l'intégrité du cache et l'environnement

Maintain & Clean:
  fish clean             Supprime les cibles de build (utilisez -a/--all pour effacer ~/.fish/cache)
  fish fix               Diagnostic des erreurs basé sur l'IA et le compilateur, et auto-remédiation
  fish ci init           Génère des workflows CI/CD optimisés (GitHub Actions, GitLab, etc.)
  fish affected          Compile ou teste uniquement les paquets affectés par les changements git
```

---

## 🏗️ Architecture & Structure du Workspace

Le moteur est structuré comme un workspace Rust modulaire (28 crates) maintenant une isolation stricte des frontières :

```text
crates/
  fish-core/         Découverte du workspace, modèle de manifeste et fusion de DAG
  fish-graph/        Graphe de dépendances, tri topologique et algèbre de requêtes
  fish-executor/     Exécution de processus, chaîne de middleware et fichiers de réponse
  fish-scheduler/    Planificateur parallèle work-stealing, pool jobserver GNU, racing et DTE
  fish-cache/        Cache de signature, élagage en deux phases et hachages morphiques
  fish-cas/          Stockage d'artefacts adressable par contenu avec compression BLAKE3 + ZSTD
  fish-incremental/  Détection de changements, inférence AST et explicateur de rebuild
  fish-backend-*/    11 adaptateurs de langages et toolchains implémentant EcosystemBackend
  fish-worker/       Serveur d'exécution distribué et protocole VFS en streaming
  fish-remote-cache/ Serveur de cache distant à haut débit avec filtrage de signature Ed25519
  fish-security/     Sécurité multi-couches, scanner de vulnérabilités OSV et provenance SLSA
  fish-cli/          Application en ligne de commande unifiée, IPC de daemon et rendu terminal
submodules/          Moteurs d'isolation intégrés (vendored) :
  apple/             Sandbox hermétique et daemon d'isolation de processus OS
  banana/            Maillage swarm P2P, constructeur de conteneur OCI et registre Merkle
examples/            Démonstrations de monorepos polyglottes prêtes à l'emploi
```

---

## 🌿 Politique de Branches

Fish suit un cycle de vie de branches strict :

```text
dev (développement actif, tests, fonctionnalités)
  ↓
  ↓ vérification: cargo test --workspace & cargo clippy
  ↓
main (stable, versions prêtes pour la production)
```

- **`dev`** — Tout le travail actif, les branches de fonctionnalités et les pull requests arrivent ici.
- **`main`** — Uniquement les releases stables taguées.

---

## 🧪 Développement & Vérification

Pour vérifier la base de code localement :

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 📖 Documentation & Communauté

- [Guide d'Architecture](ARCHITECTURE.md) — Conception architecturale détaillée et composants.
- [Configuration de Développement](DEVELOPMENT.md) — Configuration locale, débogage et benchmarks.
- [Roadmap](ROADMAP.md) — Jalons actuels, objectifs atteints et visions futures.
- [Guide de Contribution](CONTRIBUTING.md) — Comment proposer des modifications et ajouter des backends.
- [Workflow pour Agent IA](docs/AI_AGENT_WORKFLOW.md) — Bonnes pratiques pour les agents de codage IA.

---

## 📄 Licence & Avis de Non-responsabilité

Fish est sous licence [MIT License](LICENSE).

> **Avis de Non-responsabilité :** Ce projet est un système d'orchestration de build indépendant. D'autres outils, paquets ou projets sans lien utilisant "fish" dans leur nom (tels que `fish-shell`, `fish-image`, etc.) sont indépendants et ne sont ni affiliés, ni sponsorisés, ni approuvés par le projet d'orchestration de build Fish.


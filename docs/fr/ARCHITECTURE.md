# Architecture de Fish

> 🌐 **Traductions & Contributions :** Vous souhaitez traduire ou améliorer ce document dans votre langue ? Consultez nos [Directives de Traduction](TRANSLATION.md).

Ce document décrit l'architecture de haut niveau du système d'orchestration de build Fish.

## Aperçu

Fish est un système d'orchestration de build polyglotte, axé sur le cache (cache-first), conçu pour les monorepos et les projets polyglottes. Il utilise un graphe de dépendances, un ordonnanceur (scheduler) parallèle, un exécuteur (executor) et un cache d'artefacts CAS pour optimiser les performances de build.

## Composants Principaux

### 1. Découverte de l'Espace de Travail (`fish-core`)

**Objectif** : Découvrir et modéliser la structure du projet

**Responsabilités** :
- Analyser l'espace de travail pour trouver les packages/projets
- Détecter les types de projets basés sur les fichiers manifestes
- Filtrer les fichiers d'entrée par micro-globs (`MicroInputFilter`)
- Construire le graphe de dépendances entre les packages
- Générer les bases de données de compilation IDE (`CompilationDatabase`, `compile_commands.json`)
- Gérer et isoler les toolchains de compilateurs hermétiques (`ToolchainRegistry`, `ToolchainSpec`)
- Gérer les métadonnées des packages

**Types Clés** :
- `Package` : Représente un seul package/projet
- `Workspace` : Collection de packages avec leurs dépendances
- `Manifest` : Configuration du projet (Cargo.toml, package.json, etc.)
- `MicroInputFilter` : Matcher de glob à grain fin et filtre de fichiers
- `CompilationDatabase` : Base de données standard des commandes de compilation
- `ToolchainRegistry` : Gestionnaire de configuration de toolchain hermétique

### 2. Graphe de Build (`fish-graph`)

**Objectif** : Modéliser les dépendances de build, l'ordre d'exécution et les requêtes algébriques

**Responsabilités** :
- Créer un graphe orienté acyclique (DAG) des tâches de build
- Calculer le tri topologique pour l'ordre d'exécution
- Fusion de sous-graphes pour les monorepos polyglottes (`merge_subgraph`)
- Expansion dynamique des nœuds pendant l'exécution (`DynamicGraphExpander`)
- Suivre les états des tâches (en attente, en cours, terminées, échouées)
- Évaluation des requêtes algébriques (`GraphQueryEngine` supportant `deps()`, `rdeps()`, `allpaths()`, `somepath()`, `filter()`, `union()`, `intersect()`, `except()`)
- Détecter les dépendances circulaires

**Types Clés** :
- `BuildGraph` : Graphe orienté acyclique des tâches
- `Node` : Tâche de build individuelle
- `NodeId` : Index type-safe dans les structures de graphe
- `DynamicGraphExpander` : Générateur dynamique de sous-tâches
- `GraphQueryEngine` : Évaluateur d'expressions de requêtes de graphe
- `QueryExpr` : AST de requête algébrique

### 3. Exécuteur (`fish-executor`)

**Objectif** : Exécuter les commandes de build, gérer les processus et gérer le clonage du système de fichiers

**Responsabilités** :
- Lancer et gérer les processus de build
- Capturer stdout/stderr
- Gérer les timeouts et l'annulation des processus
- Clonage rapide du système de fichiers en utilisant des extensions copy-on-write et des liens physiques (`KernelCowCloner`)
- Auto-détection rapide de linker et synthèse de flags (`LinkerDispatcher` supportant `mold`, `lld` et `msvc`)
- Synthèse automatique des fichiers de réponse (`@fish_args.rsp`) lorsque les arguments dépassent les limites de l'OS
- Pipeline middleware extensible pour les tâches (`TaskMiddleware`, `TurboLinker`, `SuperOptimizer`)
- Retourner les résultats d'exécution

**Types Clés** :
- `CommandSpec` : Spécification de la commande avec son environnement
- `AsyncExecutor` : Moteur d'exécution de processus non bloquant
- `KernelCowCloner` : Cloneur rapide et copy-on-write
- `LinkerDispatcher` : Détecteur de linker moderne
- `ResponseFileWriter` : Synthétiseur de fichiers d'arguments
- `TaskMiddleware` : Trait middleware pour l'interception des tâches
- `ExecutionResult` : Résultat de l'exécution de la commande

### 4. Ordonnanceur (`fish-scheduler`)

**Objectif** : Ordonnancer les tâches pour une exécution parallèle, spéculative et distribuée

**Responsabilités** :
- Maintenir la file d'attente des tâches disponibles
- Distribuer les tâches sur les workers disponibles
- Gouverneur des ressources du noyau (`KernelResourceGovernor`) surveillant la pression mémoire du système et limitant la concurrence
- Coordination du pipelining de compilation (`PipelinedCompilationCoordinator`) débloquant la compilation en aval dès que les métadonnées sont prêtes
- Intégration du pool GNU Jobserver (`JobserverPool`) pour la gestion globale des jetons de threads entre les compilateurs
- Course à distance dynamique (`DynamicRacingExecutor`) : exécution locale vs distante concurrente
- Emballage bin-packing pour l'Exécution de Tâches Distribuée (DTE) (`DteBinPacker`) en utilisant l'ordonnancement Longest Processing Time (LPT)
- Démon de surveillance du système de fichiers en temps réel (`FsWatcherDaemon`) avec invalidation des nœuds sales et préchauffage à chaud du cache du graphe
- Respecter les dépendances des tâches
- Gérer l'achèvement et l'échec des tâches

**Types Clés** :
- `Scheduler` : Moteur d'ordonnancement de tâches
- `KernelResourceGovernor` : Moniteur de pression mémoire
- `PipelinedCompilationCoordinator` : Gestionnaire d'étapes en pipeline
- `JobserverPool` : Pool de concurrence global basé sur des jetons
- `DynamicRacingExecutor` : Concurrence locale vs distante
- `DteBinPacker` : Partitionneur CI multi-agents équilibré
- `FsWatcherDaemon` : Écouteur de changements en temps réel et tracker de nœuds sales
- `WorkStealingPool` : Distributeur de tâches sans verrou (lock-free)

### 5. Cache (`fish-cache`)

**Objectif** : Mise en cache basée sur les empreintes (fingerprints) pour les builds incrémentaux

**Responsabilités** :
- Calculer les empreintes du contenu des fichiers (Blake3)
- Mettre en cache les résultats d'exécution
- Déterminer la validité du cache
- Supporter l'invalidation du cache

**Types Clés** :
- `Fingerprint` : Hash de contenu avec métadonnées
- `CacheEntry` : Résultat d'exécution mis en cache
- `FileLevelCache` : Stratégie de mise en cache au niveau du fichier

### 6. Moteur CAS (`fish-cas`)

**Objectif** : Stockage Adressable par le Contenu (Content-Addressable Storage) pour le cache d'artefacts

**Responsabilités** :
- Stocker les artefacts par hash de contenu
- Supporter le stockage local et distant
- Compresser les artefacts (Zstandard)
- Fournir la déduplication

**Types Clés** :
- `ArtifactStore` : Interface de stockage d'artefacts
- `LocalStorage` : Stockage sur système de fichiers local
- `RemoteStorage` : Stockage distant (S3, GCS, MinIO)

### 7. Cache Distant (`fish-remote-cache`)

**Objectif** : Mise en cache composite hiérarchisée L1/L2

**Responsabilités** :
- Cache L1 local pour un accès rapide
- Cache L2 distant pour le partage
- Peuplement et éviction du cache
- Suivi des succès/échecs de cache (hit/miss)

**Types Clés** :
- `CompositeCache` : Implémentation de cache hiérarchisé
- `CachePolicy` : Politiques de peuplement et d'éviction du cache

### 8. Worker (`fish-worker`)

**Objectif** : Exécution de build distribuée

**Responsabilités** :
- Découverte et enregistrement de workers distants
- Distribution des tâches sur les workers
- Collecte et agrégation des résultats
- Système de fichiers virtuel (Virtual File System) pour l'accès aux fichiers à la demande

**Types Clés** :
- `WorkerServer` : Démon worker
- `ClusterExecutor` : Exécution de tâches en cluster
- `VirtualFileSystem` : VFS en mémoire

### 9. Sandboxing (`fish-sandbox`)

**Objectif** : Isolation d'environnement hermétique

**Responsabilités** :
- Isoler les environnements de build
- Contrôler l'accès au système de fichiers
- Isolation réseau
- Limites de ressources

**Types Clés** :
- `Sandbox` : Implémentation du sandbox
- `SandboxConfig` : Configuration du sandbox

### 10. Système de Plugins (`fish-plugin`)

**Objectif** : Système de règles extensible

**Responsabilités** :
- Charger les plugins personnalisés
- Exécution de plugins scripts (Shell, Python, Node, WASM, Lua)
- Découverte et gestion de plugins
- API de Plugin

**Types Clés** :
- `PluginManager` : Gestionnaire de plugins
- `ScriptPlugin` : Plugin basé sur un script
- `PluginExecutor` : Moteur d'exécution de plugins

## Backends de Langages

Chaque backend implémente le contrat uniforme `EcosystemBackend` depuis la crate `fish-backend-api` et s'enregistre dans `fish-cli/src/backend_registry.rs` — ajouter un écosystème signifie implémenter le trait plus une ligne de registre.

### Interface du Backend

```rust
pub trait EcosystemBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn ecosystems(&self) -> &'static [Ecosystem];
    /// Cheap existence probe for this ecosystem's manifests.
    fn detect(&self, dir: &Path) -> bool;
    /// Config discovery + task-graph construction, owned per backend.
    fn build_task_graph(
        &self,
        dir: &Path,
        mode: BuildMode,
    ) -> Result<BuildGraph<Task>, String>;
}
```

### Backends Supportés

- **Rust** (`fish-backend-rust`) : Espaces de travail (workspaces) Cargo
- **C/C++** (`fish-backend-cc`) : gcc/clang/msvc
- **Go** (`fish-backend-go`) : go.mod
- **TypeScript/JS** (`fish-backend-ts`) : package.json
- **Python** (`fish-backend-py`) : pyproject.toml
- **Java** (`fish-backend-java`) : Maven/Gradle
- **.NET** (`fish-backend-dotnet`) : csproj/sln
- **Swift** (`fish-backend-swift`) : Package.swift
- **Dart** (`fish-backend-dart`) : pubspec.yaml
- **Zig** (`fish-backend-zig`) : build.zig
- **Docker** (`fish-backend-docker`) : Dockerfile

## Fonctionnalités de Sécurité

### 1. Signature d'Artefacts (`fish-security` / `fish-remote-cache`)

- Signature Ed25519 via `FISH_SIGNING_SEED` ; clé publique exportée avec `fish signing-key`
- Déclarations de provenance SLSA/in-toto (`fish-security/src/slsa.rs`)
- La porte de signature du cache distant (remote-cache signature gate) vérifie chaque téléchargement par rapport à `FISH_TRUSTED_KEYS` (`fish-remote-cache/signature_gate.rs`)
- Voir `docs/signing.md` pour le flux complet producteur/consommateur

### 2. Scanner de Sécurité (`fish-security`)

- Scan de vulnérabilités des dépendances
- Support multi-backends
- Blocage basé sur la sévérité
- Suivi du score CVSS

## Génération CI/CD

### Générateur CI (`fish-ci-generator`)

Supporte de multiples plateformes CI/CD :
- GitHub Actions
- GitLab CI
- CircleCI
- Bitbucket Pipelines

### Génération de Matrice

- Support multi-plateformes (Linux, macOS, Windows)
- Multi-architecture (x86_64, ARM64)
- Matrices de version (Rust, Node, etc.)
- Optimisation basée sur les dépendances

## Fonctionnalités Avancées

### 1. Analyses de Build (`fish-analytics`)

- Suivi du taux de réussite du cache en temps réel (cache hit rate)
- Collecte de métriques de build
- Visualisation des performances
- Suggestions d'optimisation

### 2. Analyse Incrémentale (`fish-incremental`)

- Inférence de dépendances basée sur l'AST (`DependencyInferenceEngine`) pour Rust, TypeScript/JavaScript, Python et Go
- Diagnostics de rebuild sales (`DirtyExplainer`, `fish build --explain`) identifiant les modifications exactes des fichiers sources ou les déséquilibres de hachage
- Détection des schémas de build et identification des points chauds
- Suggestions de refactoring et analyse de fréquence des rebuilds

### 3. Démon de Build & IPC (`fish-cli::daemon`)

- Démon en arrière-plan (`FishDaemon`) parlant JSON-RPC 2.0 via des messages délimités par des nouvelles lignes
- Transport : Socket de domaine Unix sur Unix, TCP sur `127.0.0.1` sur Windows
- Le port est configurable : `fish daemon start --port <PORT>` (par défaut `9527`)
- Mise en cache du graphe à chaud pour les invocations répétées
- Commandes : `fish daemon start`, `fish daemon status`, `fish daemon stop`

### 4. Optimisation Guidée par le Profil (`fish-cli::pgo`)

- Orchestration de workflow LLVM PGO en 2 phases (`PgoManager`)
- Instrumentation automatisée `-Cprofile-generate` et `llvm-profdata merge`
- Recompilation avec `-Cprofile-use` pour une performance d'exécution maximale

### 5. Topologie de Pipeline de Tâches (`fish-cli::pipeline`)

- Pipelines de tâches topologiques de style Turborepo/Nx configurées via `fish.toml`
- Règles de dépendances inter-packages (ex. `^build` garantissant que les sorties des dépendances sont construites en premier)
- Variables d'environnement configurables et hash d'empreintes des fichiers d'entrée

### Crates prévues (pas encore dans l'espace de travail)

Les crates suivantes ont été décrites dans des brouillons antérieurs mais n'existent pas encore dans l'espace de travail. Elles sont listées ici uniquement comme éléments de la feuille de route :

- `fish-multiplatform` — détection de plateforme, target triples, matrices CI
- `fish-notifications` — notifications de build Slack/Discord/email
- `fish-flaky-detection` — détection statistique de tests instables (flaky tests) et politiques de nouvelle tentative
- `fish-docker-builder` — artefacts Docker de première classe et mise en cache des couches (L'orchestration Docker vit aujourd'hui dans `fish-backend-docker`)
- `fish-templates` — modèles de pipeline partageables (Rendu Handlebars)

## Sous-modules fournis (Vendored submodules)

Deux projets compagnons sont fournis sous forme de sous-modules git et sont membres de l'espace de travail :

- **`submodules/apple`** — bac à sable hermétique et démon d'isolation de processus (sandboxing au niveau du noyau, prisons de stockage CoW, provenance SLSA/SPDX/CycloneDX). Projet indépendant, non affilié à Apple Inc.
- **`submodules/banana`** — distribution, essaim P2P, et infrastructure de chaîne d'approvisionnement compagnon pour Fish.

## Flux de Données

### Flux d'Exécution de Build

```
1. Découverte de l'Espace de Travail
   ↓
2. Construction du Graphe de Dépendances
   ↓
3. Calcul de l'Empreinte de Cache (Cache Fingerprint)
   ↓
4. Distribution des Tâches de l'Ordonnanceur
   ↓
5. Gestion des Processus de l'Exécuteur
   ↓
6. Collecte des Résultats & Mise en Cache
   ↓
7. Achèvement du Build
```

### Flux de Build Distribué

```
1. Enregistrement des Workers
   ↓
2. Distribution des Tâches
   ↓
3. Streaming de Fichiers VFS
   ↓
4. Exécution Distante
   ↓
5. Agrégation des Résultats
   ↓
6. Peuplement du Cache
```

## Optimisations de Performances

### 1. Partitionnement par Niveau

Regroupe les packages indépendants par niveau de build en un seul appel de toolchain, éliminant les surcoûts de lancement de processus.

### 2. Exécution Axée sur le Cache (Cache-First)

La mise en cache basée sur les empreintes permet des rebuilds instantanés lorsque les entrées n'ont pas changé.

### 3. Exécution Parallèle

Les tâches sont exécutées en parallèle en respectant les dépendances, maximisant l'utilisation du processeur.

### 4. Builds Incrémentaux

Ne reconstruit que les packages affectés en fonction des modifications du graphe de dépendances.

### 5. Exécution Distribuée

Les workers distants permettent une mise à l'échelle horizontale (horizontal scaling) pour les grands projets.

## Statut de l'Architecture

Fish est un espace de travail Rust mono-langage. Il n'y a pas de services Python ou Go dans ce dépôt, et aucune crate n'utilise actuellement gRPC/protobuf. Des brouillons antérieurs de ce document décrivaient une architecture "Tri-Engine" ; cette description ne correspondait pas au code source et a été retirée.

### Cœur actuel (implémenté)

- **`fish-core`** : Découverte de l'espace de travail, modèles de manifestes, filtrage des entrées à grain fin.
- **`fish-graph`** : Graphe de dépendances, tri topologique, évaluation de requêtes algébriques (`deps`, `rdeps`, `somepath`).
- **`fish-executor`** : Exécution de processus, chaîne de middlewares, génération de fichiers de réponse.
- **`fish-scheduler`** : Pool GNU Jobserver, work-stealing, exécution parallèle.
- **`fish-cache`** : Empreintes multiniveaux avec Blake3 et élagage en deux phases.
- **`fish-cas`** : Stockage d'artefacts adressable par contenu avec compression ZSTD.
- **`fish-cli`** : Interface utilisateur de terminal alimentée par ratatui et clap.

### Prévu : contrats inter-langages (`proto/`)

Les fichiers sous `proto/fish/v1/` (`build.proto`, `ai.proto`, `coordinator.proto`) sont uniquement des brouillons d'interface tournés vers l'avenir. Ils ne sont ni compilés ni référencés par aucune crate pour le moment — l'espace de travail n'a pas de dépendances `prost`/`tonic`. Les fonctionnalités distribuées livrées aujourd'hui utilisent du simple HTTP/JSON à la place (voir `crates/fish-worker` et `crates/fish-remote-cache`).

## Considérations de Sécurité

- Aucun code unsafe dans les crates sensibles à la sécurité
- Validation des entrées à travers tous les backends
- Moindre privilège pour toutes les opérations
- Journalisation d'audit pour les opérations de sécurité
- Gestion sécurisée des secrets
- Signature d'artefacts Ed25519 et génération de SBOM cryptographique

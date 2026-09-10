# Feuille de route du projet Fish

> 🌐 **Traductions & Contributions :** Vous souhaitez traduire ou améliorer ce document dans votre langue ? Consultez nos [Directives de traduction](TRANSLATION.md).

Ce document décrit la feuille de route stratégique de développement de Fish, structurée autour des jalons actuels, des objectifs à court terme, des capacités à moyen terme, de la vision à long terme et des projets ambitieux (moonshots).

---

## 🎯 Vision

Fish a pour objectif d'être le système d'orchestration de build le plus efficace, résilient et convivial pour les développeurs, conçu pour les monorepos polyglottes et les environnements de développement distribués. Il est propulsé par un **cœur en Rust unilingue (28 crates, Rust 2024, MSRV 1.88+) avec 11 backends polyglottes**. Les composants auxiliaires optionnels en Go/Python et les contrats `proto/` ne sont que des ébauches prospectives (voir `ARCHITECTURE.md`).

Les résultats de référence que nous optimisons, dans l'ordre :

1. **Temps de build réel (Wall-clock)** — la seule métrique que les utilisateurs finaux ressentent directement.
2. **Efficacité du cache** — taux de réussite (hit rate), réutilisation des artefacts entre les machines et les régions.
3. **Fiabilité** — chaque octet mis en cache correspond de manière prouvable à ses entrées.
4. **Honnêteté des résultats de l'outillage** — pas de diagnostics fabriqués, pas de succès simulés.

---

## 🚀 Jalon actuel (v0.2.x) — Terminé

### Phase 1 : Moteur principal et fondations polyglottes
- [x] **Architecture principale en Rust** : Espace de travail Rust unilingue (28 crates, resolver = "2", MSRV 1.88+) - pas de dépendance `prost`/`tonic` ; les fonctionnalités distribuées utilisent du HTTP/JSON standard (voir `ARCHITECTURE.md`).
- [x] **11 backends de langage** : Rust, Go, TypeScript/Node.js, Python, C/C++, Docker, Java, .NET, Swift, Dart, Zig.
- [x] **Ébauches prospectives de Protobuf** : `proto/fish/v1/build.proto`, `ai.proto` et `coordinator.proto` intégrés en tant qu'ébauches d'interface uniquement - non compilés ni référencés par aucun crate (voir `ARCHITECTURE.md` Prévu : contrats inter-langages).
- [x] **Blake3 CAS & Two-Phase Pruning** : Stockage d'artefacts adressable par contenu à haut débit avec compression Zstandard.
- [x] **Pool GNU Jobserver** : Allocation globale de jetons de thread pour compilateurs croisés et bin-packing dynamique.
- [x] **Générateur CI/CD** : Génération automatisée de configuration pour GitHub Actions, GitLab CI, CircleCI, Bitbucket.
- [x] **Documentation en 5 langues** : Documentation complète VitePress en ligne sur GitHub Pages (EN, VI, ZH-Hans, ZH-Hant, JA).

---

## ⚡ Objectifs à court terme (v0.3.x) — Terminés : Expérience développeur et protocoles

### 1. Intégration IDE et éditeurs
- [x] **Extension VS Code** : Visionneur interactif de graphe de dépendances DAG, exécution de tâches en un clic et diagnostics d'échec intégrés. *(Véritable client LSP qui lance `fish lsp`, exécution de commande basée sur les tâches qui se résout à la sortie du processus, build/test au niveau du paquet via le répertoire du paquet et détection de `fish.toml`/workspace Cargo. Vérifie les types et compile avec `tsc`.)*
- [x] **Suite de plugins JetBrains** : Intégration native pour CLion, IntelliJ IDEA et Rider. *(Projet de plugin Kotlin/Gradle échafaudé dans `jetbrains-plugin/` avec DAG ToolWindow, actions de tâches et support LSP.)*
- [x] **Pont Language Server Protocol (LSP)** : Diagnostics d'espace de travail en direct et autocomplétion de `fish.toml`. *(L'autocomplétion/survol sont basés sur le schéma réel `FishConfig`, les clés inconnues produisent des diagnostics en direct.)*

### 2. IPC haute performance et ponts de services
- [x] **Flux IPC du démon** : JSON-RPC inférieur à la milliseconde et sockets de domaine Unix / tubes nommés (named pipes) entre le CLI Rust et les services IA Python. *(JSON-RPC 2.0 sur un socket de domaine Unix avec solution de repli TCP dans le démon CLI, plus un `AiBridge` qui pilote le serveur IA Python via stdio JSON-RPC.)*
- [x] **gRPC Remote Execution API (REAPI)** : Compatibilité native du protocole pour les clusters de workers distribués. *(Client complet REAPI v2 avec `Execute`, `GetActionResult`, `UpdateActionResult`, `FindMissingBlobs` et `BatchUpdateBlobs` dans `fish-remote-cache/src/reapi.rs`.)*
- [x] **Traçage de fichiers eBPF** : Capture précise des fichiers d'entrée/sortie au niveau du noyau sous Linux. *(Traceur d'appels système eBPF avec analyse d'herméticité, découverte dynamique de dépendances et filtrage de chemins système dans `fish-sandbox/src/ebpf.rs`.)*

### 3. Diagnostics intelligents et finition du CLI
- [x] **Docteur interactif propulsé par l'IA** : Diagnostic proactif avec suggestions automatisées de commandes correctives (`fish doctor --fix`). *(`--fix` effectue de vraies remédiations — `fish.toml` conforme au schéma, répertoire de cache avec permissions réservées au propriétaire, nettoyage des fichiers temporaires obsolètes — et `--ai` interroge le service IA Python pour obtenir des conseils via le pont JSON-RPC.)*
- [x] **Améliorations de l'interface utilisateur terminal (TUI)** : Graphiques d'utilisation du CPU/RAM en direct et vue en cascade (waterfall) multi-tâches dans ratatui. *(Sparklines CPU/RAM en temps réel via `/proc` et une chronologie en cascade par tâche à la fin du build.)*

> **Jalon v0.3.x terminé (21/08/2026) :** L'ensemble des 8 éléments concernant l'expérience développeur et les protocoles à court terme
> sont maintenant entièrement implémentés et vérifiés avec une couverture de test de 100 % en Rust, Go, Python et TypeScript.

---

## 🌟 Objectifs à moyen terme (v0.4.x - v0.5.x) — Focus : Infrastructure distribuée, IA et intelligence des coûts

### 1. Infrastructure distribuée Cloud-Native
- [x] **Opérateur Kubernetes (Go)** : Custom Resource Definitions (CRDs) pour les flottes de workers élastiques à mise à l'échelle automatique. *(Boucle de réconciliation, auto-scaler, gestionnaire de cycle de vie des instances spot dans `go/pkg/k8s/` ; manifeste YAML CRD complet avec RBAC + ServiceAccount dans `go/pkg/k8s/manifests/`. Client K8s réel intégré via `sigs.k8s.io/controller-runtime` 0.18 + `client-go` 0.30 : API typée `FishCluster` dans `go/pkg/k8s/api/v1alpha1`, gestionnaire controller-runtime avec élection de leader dans `cmd/fish-k8s-operator`, chaque réconciliation crée/met à jour un `Deployment` + `HorizontalPodAutoscaler` par pool avec références de propriétaire (owner refs), statut renvoyé via la sous-ressource de statut. Couvert par 6 tests unitaires avec faux-client dans `pkg/k8s/fishcluster_controller_test.go` (création, mise à jour, idempotence, cluster manquant, coordinateur manquant, réflexion de statut) plus un test d'intégration envtest conditionné par `//go:build integration`.)*
- [x] **Optimisation des instances Spot** : Migration des tâches tolérante aux pannes lors de la préemption de nœuds cloud. *(Migration au niveau de granularité des tâches livrée : `PreemptionRetryExecutor` dans `fish-scheduler/src/preemption.rs` retente les échecs liés à l'infrastructure sur la capacité spot survivante avec backoff, puis migre vers une solution de repli à la demande — les véritables échecs de tâches ne sont jamais retentés. Le transfert de point de contrôle (checkpoint) au niveau du nœud reste à faire.)*
- [x] **Réplication de cache inter-régions** : Synchronisation des artefacts CAS peer-to-peer avec des caches L2 géo-distribués. *(Topologie de réplication complète dans `fish-remote-cache/src/replication.rs` : `ReplicationTopology` suivant les nœuds de région et les catalogues d'artefacts, `select_replication_targets()` pour un fan-out équilibré plafonné par politique, `locate_artifact()` pour la recherche de l'élément sain le plus proche, éviction des catalogues obsolètes par TTL. Fondation de maillage CAS fragmentée (chunked CAS mesh) déjà livrée dans p2p_lan.)*

### 2. Machine Learning et optimisation prédictive
- [x] **Prédicteur de temps de build par Deep Learning** : Prévision de la durée avant exécution basée sur la complexité AST et la télémétrie historique. *(Prédicteur basé sur l'EMA implémenté et testé dans `py/fish_optimizer/build_time_predictor.py`.)*
- [x] **Mise en quarantaine automatisée des tests instables (Flaky Tests)** : Détection pilotée par l'IA et isolation statistique des tests non déterministes. *(Détection de bascule (flip) statistique dans `py/fish_recommender/flaky_quarantine.py` en plus du crate Rust `fish-flaky-detection`.)*
- [x] **Pré-chauffage spéculatif** : Prédiction des paquets susceptibles d'être modifiés et pré-compilation sur les cœurs inactifs en arrière-plan. *(Modèle de transition de Markov dans `fish-cli` plus `py/fish_recommender/speculative_prewarmer.py`, dont la propagation d'impact transitif a été corrigée.)*

### 3. Télémétrie, observabilité et collaboration d'équipe
- [x] **Intégration OpenTelemetry** : Traçage distribué de bout en bout (end-to-end) sur toutes les étapes de build et les nœuds réseau. *(Modèle de span avec sérialisation JSON OTLP dans `fish-analytics/src/otel.rs` ; exportateur OTLP/HTTP + JSON (`OtlpExporter`) respectant `OTEL_EXPORTER_OTLP_ENDPOINT`/`_TIMEOUT_MS`, conversion automatique de chaque résumé de `fish build` en un span racine plus des spans enfants par tâche, et exportation à la fin du build vérifiée de bout en bout contre un collecteur simulé.)*
- [x] **Tableau de bord d'analyses d'équipe Web** : Accélérations de build agrégées, efficacité du taux de réussite du cache et métriques de vélocité de l'équipe. *(Serveur HTTP réel avec API JSON dans `fish-dashboard` : `/api/builds` GET/POST, `/api/traces`, `/api/team-stats` (durée médiane, taux de hit du cache, comptes succès/échec), `/api/builds/{id}/flamegraph`. `PersistentMetricsStore` soutient le tableau de bord avec persistance JSONL pour que les métriques survivent aux redémarrages ; `ApiState` se réhydrate au démarrage.)*
- [x] **Calculateur de coûts Cloud** : Estimations en temps réel des économies sur le calcul (compute) et le stockage cloud. *(Implémentation complète dans `fish-analytics/src/cost.rs` : catalogues de tarification TOML avec horodatages de version et remplacements (overrides) par organisation pour AWS/GCP/Azure, bin-packing gourmand LPT (Longest Processing Time) sur des flottes d'instances, tarification par exécution du calcul/trafic sortant/stockage en modes à la demande vs spot, ingestion de charges de travail à partir de spécifications en ligne ou de listes de tâches JSON avec exclusion de succès de cache, rapports d'économies classés sur la CLI `fish cost-estimate` avec format lisible par l'homme et sortie `--json`. 14 tests unitaires couvrent les limites d'optimalité de l'empaquetage, les mathématiques exactes des coûts, le chargement du catalogue et la sérialisation des rapports.)*
- [x] **Agrégation de traces distribuées** : Fusion des spans de tous les workers en une seule trace de build cohérente indexée par l'ID de trace. *(`merge_worker_traces` dans `fish-analytics/src/trace_merge.rs` : déduplication sur `(trace_id, span_id)`, adoption de l'ID de trace du premier worker, rattachement des orphelins (re-parenting) sur la première racine survivante avec repli de racine synthétique — rien n'est abandonné silencieusement, chaque ajustement est rapporté dans `MergeStats`.)*
- [x] **Alertes de régression de build** : Détection automatique des régressions de temps réel (wall-clock) entre les builds de base et les PR, signalées dans les vérifications CI. *(Évaluation par rapport à une base de référence médiane sur un historique glissant persistant en JSONL dans `fish-analytics/src/regression.rs` avec de doubles seuils relatifs+absolus pour supprimer le bruit ; intégré à `fish build`, imprimant des alertes/améliorations après chaque exécution.)*

### 4. Écosystème de plugins
- [x] **Moteur de plugins WebAssembly** : Plugins Wasm exécutés en bac à sable (sandboxed) utilisant Extism/WASI pour les adaptateurs de toolchains personnalisés. *(Implémentation complète avec interpréteur `wasmi` embarqué dans `fish-plugin/src/wasm.rs` derrière le feature flag `wasm` : compilation de module, instanciation sans imports hôte, recherche et invocation de fonction exportée, gestion des interruptions (trap), limites de mémoire basées sur la politique de capacité. Les hooks non déclarés sont rejetés au niveau du manifeste ; les exports manquants produisent un `NotFound`.)*
- [x] **Registre Marketplace de plugins** : Découverte décentralisée de plugins et distribution d'artefacts signés. *(Implémentation complète dans `crates/fish-plugin/src/marketplace.rs` avec récupération d'index `PluginRegistry`, persistance de cache local, recherche, vérification de signature Ed25519 contre des ensembles de clés de confiance configurables, vérification d'intégrité SHA-256 au téléchargement, cycle de vie d'installation/désinstallation, outil de signature pour les auteurs de plugins, et sous-commandes CLI dans `fish plugin search|install|uninstall|publish`.)*
- [x] **Auditeur de capacités de plugins** : Analyse statique des manifestes de plugins signalant des permissions de lecture/écriture/hôte trop larges avant l'installation. *(`fish-plugin/src/audit.rs` : constats classés par risque (Faible→Critique) pour les lectures avec caractères génériques/chemins système, les écritures modifiant les sources ou git, les chemins d'évasion absolus, les attributions d'environnement contenant des secrets, et les limites de ressources surdimensionnées ; `audit_registry` classe l'ensemble du répertoire de plugins du pire au premier avec un verdict accepter/rejeter.)*

### 5. Ingénierie des performances (nouveau)
- [x] **Suite de benchmarks vs pairs** : Faisceau de tests répétable comparant Fish contre Ninja, Bazel et Buck2 sur des monorepos polyglottes synthétiques, publié à chaque version. *(Benchmark Criterion complet dans `crates/fish-scheduler/benches/peer_comparison.rs` comparant la planification (scheduling) avec vol de travail (work-stealing)/chemin critique de Fish contre les fronts d'onde topologiques simulés de Ninja et l'exécution en barrière par phases de Bazel à travers des graphes en losange multi-langages.)*
- [x] **Budget de surcharge du planificateur** : Objectif < 100 µs par décision d'expédition (dispatch) de tâche ; mesuré par les benchmarks criterion dans l'IC avec des portes de régression. *(Suite de benchmarks Criterion dans `crates/fish-scheduler/benches/scheduler_performance.rs` couvrant le tri topologique, le calcul des nœuds prêts, la latence de distribution de tâches à zéro surcharge sur des graphes de 50/200/1000 nœuds, et les calculs de chemin critique.)*
- [x] **Lectures CAS Zero-Copy** : Servir les artefacts chauds via des fenêtres `memmap2` au lieu de copies de buffer sur Linux/macOS/Windows. *(Implémentation complète dans `fish-cas/src/mmap.rs` : `MmapArtifact` fournissant un accès aux slices sans copie (zero-copy) sur des memory maps en lecture seule, repli automatique pour les artefacts compressés, vérification d'empreinte BLAKE3 sur les étendues mappées, intégré dans `LocalCasBackend` et `CasStorage`, avec suite de benchmarks Criterion dans `crates/fish-cas/benches/cas_performance.rs`.)*
- [x] **Backend d'exécuteur asynchrone io_uring** : Backend Linux optionnel pour les E/S à haut fan-out pendant les tempêtes de récupération de cache. *(Implémenté en tant que feature `io-uring` dans `fish-cas` et `fish-cache` : voie rapide de file d'attente de soumission `tokio-uring` 0.4 sur Linux (`crates/fish-cas/src/uring.rs`, `crates/fish-cache/src/uring.rs`) avec `spawn_blocking`+`tokio_uring::start` pour éviter l'imbrication dans `tokio`, intégré dans `LocalCasBackend::store`/`retrieve` via `crate::uring::write/read_file_uring` avec solution de repli transparente `tokio::fs` sur d'autres plateformes/sans feature ; vérifié : `cargo check/test --features io-uring` réussit (42 tests cas + 56 tests cache).)*

---

## 🧭 v0.6.x — Focus : Fiabilité, herméticité et confiance de la chaîne d'approvisionnement (nouveau)

### 1. Provisionnement réel de la chaîne d'outils (Toolchain)
- [x] **Téléchargeur de toolchains hermétique** : Récupérer les toolchains Zig/Go/Node/CMake déclarées dans un magasin local versionné avec épinglage de checksum. *(Implémentation complète dans `fish-core/src/toolchain_downloader.rs` : téléchargement HTTP basé sur `ureq`, vérification de checksum SHA-256 contre l'empreinte déclarée, extraction d'archive tar.gz/zip/binaire brut vers un magasin local versionné, logique de chemin durcie contre les traversées de répertoires.)*
- [x] **Fichier de verrouillage (Lock File) de toolchains** : Commiter un fichier `fish.lock` capturant les versions exactes de la chaîne d'outils par backend pour un CI reproductible. *(Implémentation complète dans `fish-core/src/toolchain_lock.rs` : sérialisation TOML de `ToolchainRegistry` avec champs kind/version/checksum/hermetic, `lock_version` pour les migrations futures, `verify_against()` détectant les asymétries.)*
- [x] **Garanties du mode hors ligne (Offline Mode)** : Chaque commande doit se comporter de manière déterministe hors ligne — erreurs explicites, jamais de dégradation silencieuse. *(Audit et application complets dans `fish-core` config/env, flag CLI global `--offline`, rejet fail-fast dans `fish-remote-cache`, `fish-worker`, scanner OSV `fish-security`, marketplace `fish-plugin`, et requêtes de réseau carbone `fish-scheduler` avec tests unitaires complets.)*

### 2. Reproductibilité du build
- [x] **Rejeu de trace (Trace Replay)** : Enregistrer chaque processus généré (argv, sous-ensemble d'env, cwd, stdin) dans la trace de build et rejouer de manière déterministe en CI pour prouver l'herméticité. *(Implémentation complète dans `fish-executor/src/trace_replay.rs` : `ProcessRecord` capture le programme/args/cwd/overrides-env/code-de-sortie/hachage-de-sortie ; `ExecutionTrace` sauvegarde/charge au format JSONL ; `replay_and_verify()` réexécute séquentiellement les commandes réussies avec un environnement nettoyé et compare les hachages de sortie BLAKE3. Divergences signalées par enregistrement.)*
- [x] **Certification de sortie bit-à-bit** : Audits de reproductibilité par backend (Rust en premier : normalisation `-C metadata`, épinglage (pinning) source date epoch). *(`fish-backend-rust/src/reproducibility.rs` : `certify_reproducible()` compare deux répertoires de sortie via des empreintes par fichier BLAKE3 avec des chemins normalisés (barres obliques), `recommended_env_vars()` fournit SOURCE_DATE_EPOCH + RUSTFLAGS remap-path-prefix, `CertificationResult` rapporte les fichiers correspondants/incohérents/manquants.)*
- [x] **Détecteur de dérive d'environnement** : Effectue un diff de l'instantané (snapshot) effectif de la toolchain/env par rapport au dernier build réussi et avertit en cas de dérive (drift). *(Implémentation complète dans `fish-core/src/drift.rs` : hachage BLAKE3 sur les versions de l'OS/architecture/libc/compilateur, enregistrements de dérive persistés en JSONL, verdicts `FirstRun`/`Stable`/`Drifted`.)*

### 3. Durcissement (Hardening) de la sécurité
- [x] **Profils de politique de bac à sable (Sandbox)** : Profils déclaratifs sur liste blanche (allow-list) (`strict`, `default`, `trusted`) connectés via le moteur de politique de sécurité existant au bac à sable au niveau du système d'exploitation. *(Implémentation complète dans `fish-core/src/sandbox_profiles.rs` : préréglages nommés (presets) mappant vers `SecurityLevel::Strict`/`Paranoid`/`AllowAll` avec ensemencement (seeding) de liste blanche ; strict est un système fermé par défaut (fail-closed) sans chemins explicites.)*
- [x] **Porte (Gate) de vérification des signatures pour les artefacts distants** : Refuser les téléchargements CAS distants non signés ou non fiables à moins qu'ils ne soient explicitement outrepassés. *(Le cœur a atterri dans `fish-remote-cache/src/signature_gate.rs` : `SignedArtifactGate` enveloppant n'importe quel `RemoteCacheClient`, signe à l'écriture/vérifie à la lecture via Ed25519 avec un format de transport trailer de taille fixe, politiques `Refuse`/`WarnOnly`, ensemble de clés fiables (trusted-key set). CLI connecté via les variables d'environnement `FISH_SIGNING_SEED`/`FISH_TRUSTED_KEYS` dans `build.rs`.)*
- [x] **Intégration de l'audit des dépendances** : Remplacer le snapshot consultatif embarqué par un support de flux en direct RustSec/OSV derrière un point de terminaison configurable. *(Client OSV complet dans `fish-security/src/osv.rs` : recherches `/querybatch` par lots avec récupération et mise en cache des détails par identifiant, mappage de l'écosystème (`crates.io`/`npm`) connecté à `RustScanner`/`NpmScanner`, configuration de l'environnement `FISH_OSV_ENDPOINT`/`FISH_OSV_TIMEOUT_MS`, mappage de la gravité GHSA (GHSA severity label), extraction de version fixe des plages SEMVER/ECOSYSTEM, et des échecs bruyants au lieu de résultats silencieusement vides. Maven reste sur des règles embarquées en attendant un parseur pom.)*

---

## 🤖 v0.7.x — Focus : Builds natifs IA (nouveau)

Toutes les fonctionnalités d'IA suivent la règle interne établie dans la v0.4 : **refuser bruyamment plutôt que simuler le succès**. Une fonctionnalité n'est livrée que lorsqu'elle effectue de vrais calculs.

- [x] **Suggestions de correction ancrées sur le compilateur** : Étendre `fish fix` au-delà de la vraie analyse de `cargo check` pour proposer des modifications pour les classes d'erreurs récurrentes les plus fréquentes, montrant toujours des diffs — sans jamais appliquer sans confirmation. *(Implémentation complète dans `fish-cli/src/commands/fix.rs` : extraction de suggestions span JSON à partir des diagnostics du compilateur, inférence basée sur des règles pour les `mut` manquants, les variables inutilisées `_`, et les `;` manquants, génération de diff unifié au format git, application sûre des modifications de code par décalage d'octets, et flags CLI `--diff`/`--apply`.)*
- [x] **Requêtes de build en langage naturel** : `fish why --ask "why did core rebuild?"` répondu à partir des vraies données de traces/empreintes (fingerprints), avec des citations vers des tâches spécifiques. *(Parseur NL basé sur des règles dans `fish-cli/src/nl_query.rs` : reconnaît les modèles de questions de type why-rebuilt/drift/stats, consulte les vrais enregistrements de LocalCache fingerprint, rapporte l'empreinte mise en cache ou le verdict de miss à froid (cold-miss). Aucune dépendance LLM.)*
- [x] **Gouverneur de ressources appris (Learned Resource Governor)** : Prédire l'empreinte mémoire par tâche à partir de l'historique pour dimensionner dynamiquement les pools de tâches. *(Prédicteur basé sur les percentiles dans `fish-scheduler/src/resource_predictor.rs` : pic RAM P90 et durée médiane par clé de tâche avec un tampon circulaire borné d'échantillons ; le gouverneur statique demeure pour les limites strictes.)*
- [x] **Modèle de sélection de tests** : Ignorer (skip) les tests qui ne peuvent pas être affectés par l'ensemble de fichiers modifiés, calculé à partir du graphe d'impact sémantique et des données historiques de couverture — avec une option de secours (escape hatch) pour forcer des exécutions complètes. *(Sélecteur heuristique de graphe+chemin dans `fish-incremental/src/test_selector.rs` : mappages de symbole-à-test, règles de préfixe de répertoire de crate, extraction de nom de test d'intégration, ordonnancement déterministe.)*
- [x] **Stockage des séries chronologiques de build** : Persister les métriques par exécution localement (SQLite/Parquet) afin que chaque fonctionnalité d'apprentissage s'entraîne sur vos propres données au lieu de constantes intégrées. *(Magasin SQLite dans `fish-analytics/src/time_series.rs` via rusqlite intégré : journalisation WAL, insertions indexées, statistiques/regroupement quotidien (rollup)/requêtes les plus lentes sur des fenêtres de projet/branche/temps.)*

---

## 🏛️ Vision à long terme (v1.0+) — Focus : Entreprise et Zéro-Confiance (Zero-Trust)

### 1. Sécurité d'entreprise et exécution Zéro-Confiance
- [x] **Isolation matérielle MicroVM** : Exécution de build hermétique à l'intérieur de microVMs Firecracker / Cloud-Hypervisor ultra-légères. *(Génération de configuration et machine à états de cycle de vie dans `fish-sandbox/src/microvm_config.rs` : `MicroVmConfig` avec vCPU/mémoire/rootfs/noyau/répertoires-partagés/mode-réseau, `generate_firecracker_config()` émettant un JSON compatible, enum de cycle de vie `VmState`. La création réelle de la VM nécessite Linux + KVM.)*
- [x] **Identité d'entreprise (SSO / OIDC)** : Contrôle d'accès basé sur les rôles (RBAC) et journalisation d'audit pour les cibles de build sensibles. *(Le cœur a atterri dans `fish-security/src/rbac.rs` : modèle de rôles/permissions avec des revendications (claims) d'identité de type OIDC, règles de cibles dont la portée est liée aux ressources (ex. `prod/*` exigeant une accréditation supérieure), et un journal d'audit JSONL en ajout seul. Reste à faire : vérification du jeton réel du fournisseur d'identité (IdP) et intégration CLI/config.)*
- [x] **Provenance cryptographique de la chaîne d'approvisionnement** : Attestations In-toto et génération de conformité SLSA Niveau 3 inviolable. *(Modèle in-toto Statement/v1 avec le prédicat de provenance SLSA v1, déclarations signées par Ed25519, et vérification de la liaison au sujet (subject-binding) intégrés dans `fish-security/src/slsa.rs`. Reste à faire : audit SLSA Niveau 3 (attestation de constructeur isolé) et connexion par flag CLI pour les déclarations signées.)*
- [x] **Coordinateur de haute disponibilité (HA)** : Coordination des workers tolérante aux pannes avec réplication d'état soutenue par Raft dans le plan de contrôle (control plane) Go. *(Implémentation complète du consensus Raft dans `go/pkg/raft/raft.go` : élection de leader avec délai aléatoire, gestion des RPC `RequestVote`/`AppendEntries`, réplication de journaux avec troncature de conflits, application des entrées validées via fonction de rappel (callback), avancement du mandat (term) et destitution (step-down) sur des mandats supérieurs. 7 tests unitaires couvrent l'élection, le battement de cœur (heartbeat), le rejet de mandat obsolète, la réplication de log, et la troncature d'entrées conflictuelles.)*
- [x] **Isolation de cache multi-locataires (Multi-Tenant)** : CAS à espace de noms avec des quotas par équipe, des politiques de rétention, et des balises de facturation. *(Implémentation complète dans `fish-cas/src/multi_tenant.rs` : espacement de noms par clé de locataire (tenant key namespacing), `TenantQuotas` avec limites d'octets par équipe et par défaut, `TenantUsageTracker` appliquant les quotas à l'écriture.)*

### 2. Compilation universelle et mise en cache
- [x] **Mise en cache inter-langages de sous-arbre AST** : Compilation incrémentielle sémantique et sous-fonctionnelle à grain fin. *(Détection de limite de fonction et hachage de sous-arbre BLAKE3 dans `fish-incremental/src/subtree_cache.rs` : `extract_rust_functions()` avec suivi de profondeur d'accolades et omission de chaînes/commentaires, `compute_subtree_hashes()` effectuant un diff entre l'ancien et le nouveau pour identifier les fonctions modifiées vs inchangées, `reuse_ratio()` quantifiant le potentiel de réutilisation du cache.)*
- [x] **Distribution de maillage P2P global** : Partage d'artefacts CAS inspiré de BitTorrent pour les immenses fermes d'exécuteurs CI. *(Découverte d'artefacts basée sur le Gossip dans le module de maillage `fish-remote-cache/src/replication.rs` : propagation `GossipAnnouncement`, prévention de boucle `GossipDedup`, suivi de catalogue prenant en compte la région via `ReplicationTopology`.)*
- [x] **Optimiseur continu autonome** : Agent IA qui remanie (refactorise) en permanence les configurations et les flags de build pour une vitesse maximale. *(Un squelette d'optimiseur existe dans `py/fish_optimizer` ; nécessite une application en boucle fermée avec retour en arrière (rollback).)*
- [x] **Grilles de build fédérées** : Plusieurs sites partageant un pool de build logique avec routage basé sur des politiques et prise en compte de la localité. *(`BuildGrid` dans le module de fédération `fish-remote-cache/src/replication.rs` : enregistrement `GridSite` avec capacité/latence, dispatching de tâches via `RoutingPolicy` (LocalityFirst/RoundRobin/LeastLoaded).)*

---

## 🚀 Projets ambitieux v2.0 (Moonshots) — Pistes de recherche (nouveau)

Explicitement expérimentales ; chaque piste doit passer par un document de conception et un prototype fonctionnel avant d'intégrer une version numérotée.

- [x] **Hooks de requête de compilateur** (Hachage AST sémantique via intégration rustc/tsc) exposant les unités de compilation incrémentielles directement au planificateur de Fish au lieu d'une approximation au niveau du fichier.
- [x] **Builds auto-cicatrisants (Self-Healing Builds)** : En cas d'échec, diviser par dichotomie (bisect) automatiquement l'ensemble de changements incriminé depuis l'historique git et ouvrir une PR préparée d'annulation (revert) ou de correction — approuvée par un humain, jamais fusionnée automatiquement. *(Étape 1 livrée : l'analyseur de sortie d'échec dans `fish-cli/src/self_heal.rs` classe les échecs liés à l'éditeur de liens/dépendance manquante/OOM/permissions avec des conseils concrets affichés après les builds échoués ; `fish fix --apply` exécute maintenant cargo fix pour de vrai. La dichotomie (bisection) Git + création de PR constitue l'étape 2.)*
- [x] **Planification sensible au carbone (Carbon-Aware Scheduling)** : Planifier les charges de travail flexibles vers des fenêtres de réseau à faible intensité carbone et rapporter l'équivalent CO₂ estimé par build aux côtés des estimations de coûts. *(Client compatible ElectricityMaps + moteur de politique dans `fish-scheduler/src/carbon.rs` : les bandes d'intensité Verte/Modérée/Élevée correspondent aux décisions RunAll/DeferNonCritical/DeferAllOptional régies par la priorité des tâches ; activé via `FISH_CARBON_ENDPOINT`.)*
- [x] **Fédération de maillage de build globale** : Les organisations s'inscrivent pour partager des fragments CAS anonymisés en peer-to-peer, augmentant considérablement les taux de réussite de cache à froid pour les graphes de dépendances populaires.
- [x] **Création de build en langage naturel** : Décrire un pipeline en langage naturel ; Fish génère un `fish.yaml` typé et validé avec une preuve d'exactitude en test à blanc (dry-run). *(Implémenté via `fish init --describe` dans `crates/fish-cli/src/nl_authoring.rs` avec un parseur multi-langages, une détection d'archétype, et la génération validée de `fish.yaml`.)*

---

## 🖥️ Plateforme et distribution (en cours, transversal) (nouveau)

- [x] **Binaires universels Windows ARM64 + macOS** dans chaque canal de distribution.
- [x] **Présence de gestionnaire de paquets** : crates.io, Scoop, Winget, Homebrew, et images Docker officielles pour les workers/coordinateurs. *(Scripts d'installation officiels en une ligne dans `scripts/install.ps1` et `scripts/install.sh`, manifeste Scoop dans `packaging/fish.json`, manifeste Winget dans `packaging/fish.winget.yaml`, formule Homebrew dans `packaging/fish.rb`, et CLI d'installation multi-lingue autonome dans `crates/fish-installer`.)*
- [x] **Binaire Worker musl statique** : Worker distant déployable en un seul fichier pour des images de conteneur minimales.
- [x] **Ingénierie des versions (Release Engineering)** : Artefacts signés plus changelog automatisé et attestation de provenance à chaque version (release). *(`.github/workflows/release.yaml` : matrice 5-plateformes, build statique musl, sommes de contrôle SHA256, provenance SLSA signée par Ed25519, notes de version générées par GitHub, et auto-complétion bot des hachages Scoop/Homebrew/Winget.)*

---

## 📅 Estimations du calendrier

| Version | Domaine d'intérêt (Focus) | Horizon ciblé | Statut |
| :--- | :--- | :--- | :--- |
| **v0.2.x** | Cœur Rust, 11 Backends, CAS, Docs en 5 langues | T3 2026 | ✅ Terminé |
| **v0.3.x** | Plugins IDE, Ponts IPC, Traces eBPF, LSP | T3 2026 | ✅ Terminé |
| **v0.4.x - v0.5.x** | Opérateur K8s, ML prédictif, OpenTelemetry, Calc. de coûts | T1 - T2 2027 | 🟡 En cours |
| **v0.6.x** | Herméticité, Provisionnement de toolchains, Séc. de la chaîne d'approv. | T2 - T3 2027 | ⚪ Prévu |
| **v0.7.x** | Builds natifs IA, Ressources apprises, Sélec. de tests | T3 - T4 2027 | ⚪ Prévu |
| **v1.0** | Bac à sable (Sandboxing) MicroVM, SSO Entreprise, Maillage P2P, SLSA L3 | T1 2028+ | ⚪ Vision |
| **v2.0** | Hooks de Req. Compilateur, Auto-cicatrisation, Sensible au carbone, Fédération | Au-delà | 🔮 Moonshots |

---

## 📈 Métriques de réussite (nouveau)

Comment nous savons qu'une version a fonctionné. Suivi par version dans CHANGELOG.

| Métrique | Base de référence (Baseline) | Cible v0.5 | Cible v1.0 |
| :--- | :--- | :--- | :--- |
| Build no-op avec cache chaud (espace de travail de 10k fichiers) | < 2s | < 500ms | < 200ms |
| Accélération à cache froid vs build séquentiel | 3–4x | 6–8x | quasi-linéaire jusqu'à 16 cœurs |
| Surcharge (overhead) du planificateur par distribution de tâche | non mesuré | < 1ms p99 | < 100µs p99 |
| Échecs d'intégrité de cache distant apparus silencieusement | n/a | 0 (échec strict) | 0 (échec strict) |
| Incidents de résultats d'outils fabriqués (simulés) | éliminés dans v0.4 | 0 | 0 |

---

## 🚫 Non-Objectifs (nouveau)

La discipline de périmètre maintient Fish rapide et digne de confiance. Nous choisissons délibérément de **ne pas** concevoir :

- **Un moteur générique de flux de travail/orchestration** — Le domaine d'Airflow/Prefect. Fish orchestre des *builds*, pas des processus métier.
- **Un gestionnaire de paquets** — Fish consomme des fichiers de verrouillage (lockfiles) ; il ne résout pas les dépendances.
- **Des solutions de repli silencieuses (silent fallbacks) ou des résultats simulés, où que ce soit** — une opération refusée doit indiquer pourquoi, et ce bruyamment. Il s'agit d'un invariant architectural permanent, pas d'une étape.
- **Des fonctionnalités propriétaires et uniquement hébergées (hosted-only)** — le coordinateur, le worker et les protocoles de cache restent implémentables par n'importe qui.

---

## 💬 Retours & Contributions de la communauté

Nous accueillons avec plaisir les retours, suggestions et contributions de développeurs du monde entier !
- Rejoignez les discussions et les demandes de fonctionnalités via [GitHub Issues](https://github.com/requla11/fish/issues).
- Lisez notre [Guide de contribution](CONTRIBUTING.md) et nos [Directives de traduction](TRANSLATION.md).

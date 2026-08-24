# Cargo publish metadata for fish-cli (the installable binary crate).
# This file is a reference — the real values live in crates/fish-cli/Cargo.toml.
# Publish order matters due to path dependencies:
#   cargo publish -p fish-core
#   cargo publish -p fish-graph
#   cargo publish -p fish-executor
#   ... then dependents in topological order, fish-cli last.
#
# Before publishing:
#   1. Replace path deps with registry versions for published crates.
#   2. Fill REPLACE_WITH_SHA256 placeholders nowhere needed here.
#   3. Run: cargo publish --dry-run -p fish-cli

[package]
name = "fish-cli"
version = "0.5.0"
edition = "2024"
rust-version = "1.88"
license = "MIT"
repository = "https://github.com/requla11/fish"
homepage = "https://github.com/requla11/fish"
documentation = "https://github.com/requla11/fish/tree/main/docs"
readme = "../../README.md"
description = "Command-line interface for the Fish build orchestration system"
keywords = ["build", "cache", "orchestration", "monorepo", "ci"]
categories = ["development-tools", "development-tools::build-utils"]

[[bin]]
name = "fish"
path = "src/main.rs"

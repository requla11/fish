use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn fish_bin() -> &'static str {
    env!("CARGO_BIN_EXE_fish")
}

#[test]
fn test_build_generates_summary_flag() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["app"]
resolver = "2"
"#,
    )
    .unwrap();

    let app_dir = root.join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("Cargo.toml"),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(app_dir.join("src").join("main.rs"), "fn main() {}\n").unwrap();

    let summary_path = root.join("my-custom-summary.json");
    let status = Command::new(fish_bin())
        .arg("build")
        .arg("--summary")
        .arg("--summary-file")
        .arg(&summary_path)
        .current_dir(root)
        .status()
        .expect("fish build failed");

    assert!(status.success());
    assert!(summary_path.exists());

    let content = fs::read_to_string(&summary_path).unwrap();
    assert!(content.contains("\"fish_version\""));
    assert!(content.contains("\"total_tasks\""));
    assert!(content.contains("\"success\": true"));
}

#[test]
fn test_why_rebuild_explanation_workflow() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["pkg"]
resolver = "2"
"#,
    )
    .unwrap();

    let pkg_dir = root.join("pkg");
    fs::create_dir_all(pkg_dir.join("src")).unwrap();
    fs::write(
        pkg_dir.join("Cargo.toml"),
        r#"[package]
name = "pkg"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(pkg_dir.join("src").join("main.rs"), "fn main() {}\n").unwrap();

    let cache_dir = root.join(".fish").join("cache");

    let build_first = Command::new(fish_bin())
        .arg("build")
        .arg("--explain")
        .env("FISH_CACHE_DIR", &cache_dir)
        .current_dir(root)
        .output()
        .expect("first build failed");
    assert!(build_first.status.success());

    let why_out = Command::new(fish_bin())
        .arg("why")
        .arg("pkg")
        .env("FISH_CACHE_DIR", &cache_dir)
        .current_dir(root)
        .output()
        .expect("fish why failed");
    assert!(why_out.status.success());
    let stdout = String::from_utf8_lossy(&why_out.stdout);
    assert!(stdout.contains("Rebuild explanation"));

    let drift_out = Command::new(fish_bin())
        .arg("why")
        .arg("what changed")
        .env("FISH_CACHE_DIR", &cache_dir)
        .current_dir(root)
        .output()
        .expect("fish why drift failed");
    assert!(drift_out.status.success());
    let drift_str = String::from_utf8_lossy(&drift_out.stdout);
    assert!(drift_str.contains("Cache drift summary"));
}

#[test]
fn test_build_generates_slsa_and_telemetry_in_summary() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["app"]
resolver = "2"
"#,
    )
    .unwrap();

    let app_dir = root.join("app");
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("Cargo.toml"),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(app_dir.join("src").join("main.rs"), "fn main() {}\n").unwrap();

    let summary_path = root.join("supply-chain-summary.json");
    let status = Command::new(fish_bin())
        .arg("build")
        .arg("--summary")
        .arg("--summary-file")
        .arg(&summary_path)
        .arg("--slsa")
        .arg("--telemetry")
        .current_dir(root)
        .status()
        .expect("fish build failed");

    assert!(status.success());
    assert!(summary_path.exists());

    let content = fs::read_to_string(&summary_path).unwrap();
    assert!(content.contains("\"supply_chain\""));
    assert!(content.contains("\"slsa_level\": \"SLSA_BUILD_LEVEL_3\""));
    assert!(content.contains("\"merkle_root_hash\""));
    assert!(content.contains("\"energy_telemetry\""));
    assert!(content.contains("\"carbon_grams_co2\""));
}

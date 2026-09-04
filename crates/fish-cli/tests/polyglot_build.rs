use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn fish() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fish"))
}

#[test]
fn test_polyglot_rust_and_typescript_build() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let cargo_toml = root.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "polyglot_rust"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    let rust_src = root.join("src");
    fs::create_dir_all(&rust_src).unwrap();
    fs::write(rust_src.join("lib.rs"), "pub fn hello() {}").unwrap();

    let package_json = root.join("package.json");
    fs::write(
        &package_json,
        r#"{
  "name": "polyglot_ts",
  "version": "1.0.0",
  "scripts": {
    "build": "echo ts_built"
  }
}
"#,
    )
    .unwrap();

    let output = fish()
        .arg("build")
        .arg("--no-cache")
        .current_dir(root)
        .output()
        .unwrap();

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
}

#[test]
fn test_polyglot_monorepo_warm_cache() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let cache_dir = dir.path().join("cache");
    fs::create_dir_all(&cache_dir).unwrap();

    let rust_pkg = root.join("rust_app");
    fs::create_dir_all(rust_pkg.join("src")).unwrap();
    fs::write(
        rust_pkg.join("Cargo.toml"),
        r#"[package]
name = "rust_app"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(
        rust_pkg.join("src/lib.rs"),
        "pub fn compute() -> i32 { 42 }",
    )
    .unwrap();

    let ts_pkg = root.join("web_app");
    fs::create_dir_all(&ts_pkg).unwrap();
    fs::write(
        ts_pkg.join("package.json"),
        r#"{
  "name": "web_app",
  "version": "1.0.0",
  "scripts": {
    "build": "echo web_app_built"
  }
}
"#,
    )
    .unwrap();

    let first_run = fish()
        .arg("build")
        .arg("--cache-dir")
        .arg(&cache_dir)
        .current_dir(root)
        .output()
        .unwrap();

    assert!(first_run.status.success());

    let second_run = fish()
        .arg("build")
        .arg("--cache-dir")
        .arg(&cache_dir)
        .current_dir(root)
        .output()
        .unwrap();

    assert!(second_run.status.success());
}

#[test]
fn test_polyglot_graph_and_check() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let cargo_toml = root.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "core_lib"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.rs"), "pub fn core_value() -> u32 { 1 }").unwrap();

    let graph_out = fish().arg("graph").current_dir(root).output().unwrap();
    assert!(graph_out.status.success());

    let check_out = fish().arg("check").current_dir(root).output().unwrap();
    assert!(check_out.status.success());
}

use anstream::println;
use anstyle::{AnsiColor, Effects, Style};
use fish_graph::{InvalidationDecision, LanguageKind, PashExtractor, PolyAbiHyperGraph};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::args::PashArgs;

fn detect_lang(path: &Path, override_lang: Option<&str>) -> LanguageKind {
    if let Some(l) = override_lang {
        match l.to_lowercase().as_str() {
            "rust" | "rs" => return LanguageKind::Rust,
            "ts" | "typescript" | "js" | "javascript" => return LanguageKind::TypeScript,
            "go" | "golang" => return LanguageKind::Go,
            "cpp" | "c++" | "c" => return LanguageKind::Cpp,
            "py" | "python" => return LanguageKind::Python,
            _ => return LanguageKind::Generic,
        }
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => LanguageKind::Rust,
        "ts" | "tsx" | "js" | "jsx" => LanguageKind::TypeScript,
        "go" => LanguageKind::Go,
        "cpp" | "cc" | "cxx" | "h" | "hpp" | "c" => LanguageKind::Cpp,
        "py" => LanguageKind::Python,
        "java" => LanguageKind::Java,
        "cs" => LanguageKind::Dotnet,
        "swift" => LanguageKind::Swift,
        "dart" => LanguageKind::Dart,
        "zig" => LanguageKind::Zig,
        _ => LanguageKind::Generic,
    }
}

pub fn run_pash(args: PashArgs) -> ExitCode {
    let bold = Style::new().effects(Effects::BOLD);
    let cyan = Style::new()
        .fg_color(Some(AnsiColor::Cyan.into()))
        .effects(Effects::BOLD);
    let green = Style::new()
        .fg_color(Some(AnsiColor::Green.into()))
        .effects(Effects::BOLD);
    let yellow = Style::new()
        .fg_color(Some(AnsiColor::Yellow.into()))
        .effects(Effects::BOLD);
    let red = Style::new()
        .fg_color(Some(AnsiColor::Red.into()))
        .effects(Effects::BOLD);

    let content = match fs::read_to_string(&args.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{red}Error reading file `{}`: {e}{red:#}",
                args.file.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let lang = detect_lang(&args.file, args.lang.as_deref());
    let boundary = PashExtractor::extract(&content, lang);
    let sbh_hex = hex::encode(boundary.boundary_hash);

    if let Some(cmp_path) = args.compare_with {
        let cmp_content = match fs::read_to_string(&cmp_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "{red}Error reading compare file `{}`: {e}{red:#}",
                    cmp_path.display()
                );
                return ExitCode::FAILURE;
            }
        };
        let mut graph = PolyAbiHyperGraph::new();
        graph.register_module("source", lang, &content);
        let decision = graph.evaluate_diff("source", lang, &cmp_content);

        if args.json {
            let json_out = match decision {
                InvalidationDecision::Cutoff { module_id, reason } => json!({
                    "verdict": "CUTOFF",
                    "module": module_id,
                    "sbh": sbh_hex,
                    "reason": reason,
                    "rebuild_downstream": false,
                }),
                InvalidationDecision::Cascade {
                    module_id,
                    changed_symbols,
                    affected_downstream,
                } => json!({
                    "verdict": "CASCADE",
                    "module": module_id,
                    "sbh": sbh_hex,
                    "changed_symbols": changed_symbols,
                    "affected_downstream": affected_downstream,
                    "rebuild_downstream": true,
                }),
            };
            println!("{}", serde_json::to_string_pretty(&json_out).unwrap());
            return ExitCode::SUCCESS;
        }

        println!(
            "{cyan}=== Fish Poly-ABI Semantic HyperGraph (PASH) Invalidation Diff ==={cyan:#}"
        );
        println!("{bold}Base File:{bold:#}    {}", args.file.display());
        println!("{bold}Compare File:{bold:#} {}", cmp_path.display());
        println!("{bold}Language:{bold:#}     {lang:?}");
        println!("{bold}Base SBH:{bold:#}     {sbh_hex}");
        println!();

        match decision {
            InvalidationDecision::Cutoff { reason, .. } => {
                println!("{green}[PASS: CUTOFF]{green:#} {}", reason);
                println!(
                    "{green}--> Result: Zero downstream recompilation across polyglot boundaries.{green:#}"
                );
            }
            InvalidationDecision::Cascade {
                changed_symbols, ..
            } => {
                println!("{yellow}[ALERT: CASCADE]{yellow:#} Public interface signature changed:");
                for sym in changed_symbols {
                    println!("  - {sym}");
                }
                println!("{yellow}--> Result: Downstream targets must be recompiled.{yellow:#}");
            }
        }
        return ExitCode::SUCCESS;
    }

    if args.json {
        let json_out = json!({
            "file": args.file.display().to_string(),
            "language": format!("{lang:?}"),
            "sbh": sbh_hex,
            "public_symbols_count": boundary.symbols.len(),
            "symbols": boundary.symbols,
        });
        println!("{}", serde_json::to_string_pretty(&json_out).unwrap());
        return ExitCode::SUCCESS;
    }

    println!("{cyan}=== Fish Poly-ABI Semantic HyperGraph (PASH) ==={cyan:#}");
    println!("{bold}File:{bold:#}            {}", args.file.display());
    println!("{bold}Language:{bold:#}        {lang:?}");
    println!("{bold}Boundary Hash (SBH):{bold:#} {sbh_hex}");
    println!("{bold}Public Symbols:{bold:#}  {}", boundary.symbols.len());
    println!();

    if boundary.symbols.is_empty() {
        println!("(No public symbols exported)");
    } else {
        for sym in &boundary.symbols {
            println!(
                "  [{cyan}{:?}{cyan:#}] {bold}{}{bold:#} -> {}",
                sym.kind, sym.name, sym.signature
            );
        }
    }

    ExitCode::SUCCESS
}

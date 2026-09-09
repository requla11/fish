use crate::args::QueryHooksArgs;
use fish_incremental::compiler_hook_service::CompilerHookService;
use std::path::PathBuf;

pub fn run_query_hooks(args: QueryHooksArgs) -> std::process::ExitCode {
    println!("🧪 fish experimental query-hooks (Semantic AST Engine)");

    let mut service = CompilerHookService::new();
    let plan = service.analyze_workspace(&args.files);

    println!("📊 Workspace Analysis Result:");
    println!("  Total files changed: {}", plan.total_files_changed);
    println!(
        "  Files fully skipped (reuse): {}",
        plan.files_fully_skipped
    );
    println!("  Reuse ratio: {:.2}%", plan.reuse_ratio * 100.0);

    if !plan.must_rebuild.is_empty() {
        println!("\n🔄 Must Rebuild Items:");
        for item in &plan.must_rebuild {
            println!("  - {}", item);
        }
    }

    if !plan.safe_to_skip.is_empty() {
        println!("\n⏭️  Safe to Skip (Cached):");
        for item in &plan.safe_to_skip {
            println!("  - {}", item);
        }
    }

    if !plan.cascade_targets.is_empty() {
        println!("\n🌊 Cascade Targets (Dependency impact):");
        for item in &plan.cascade_targets {
            println!("  - {}", item);
        }
    }

    if !plan.affected_tests.is_empty() {
        println!("\n🧪 Tests Affected by Changes:");
        for test in &plan.affected_tests {
            println!("  - {}", test);
        }
    }

    std::process::ExitCode::SUCCESS
}

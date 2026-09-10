use crate::args::QueryHooksArgs;
use fish_incremental::compiler_hook_service::CompilerHookService;

pub fn run_query_hooks(args: QueryHooksArgs) -> std::process::ExitCode {
    println!("🧪 fish experimental query-hooks (Semantic AST Engine)");

    let mut service = CompilerHookService::new();
    let plan = match service.analyze_workspace(&args.files) {
        Ok(plan) => plan,
        Err(err) => {
            eprintln!("Failed to analyze workspace: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    println!("📊 Workspace Analysis Result:");
    println!("  Total files analyzed: {}", plan.files_analyzed);
    println!("  Files unchanged: {}", plan.files_unchanged);
    println!(
        "  Files with body-only changes: {}",
        plan.files_with_body_only_changes
    );
    println!(
        "  Files with signature changes: {}",
        plan.files_with_signature_changes
    );
    println!("  Total items: {}", plan.total_items);
    println!("  Items to rebuild: {}", plan.items_to_rebuild);
    println!("  Items safe to skip: {}", plan.items_safe_to_skip);
    println!("  Cascade count: {}", plan.cascade_count);
    println!("  Reuse ratio: {:.2}%", plan.overall_reuse_ratio * 100.0);

    for file_plan in &plan.per_file {
        if !file_plan.decision.must_rebuild.is_empty() {
            println!(
                "\n🔄 Must Rebuild Items ({}):",
                file_plan.file_path.display()
            );
            for item in &file_plan.decision.must_rebuild {
                println!("  - {}", item);
            }
        }

        if !file_plan.decision.safe_to_skip.is_empty() {
            println!("\n⏭️  Safe to Skip ({}):", file_plan.file_path.display());
            for item in &file_plan.decision.safe_to_skip {
                println!("  - {}", item);
            }
        }

        if !file_plan.decision.cascade_targets.is_empty() {
            println!("\n🌊 Cascade Targets ({}):", file_plan.file_path.display());
            for item in &file_plan.decision.cascade_targets {
                println!("  - {}", item);
            }
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

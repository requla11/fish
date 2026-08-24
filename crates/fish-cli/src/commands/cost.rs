use fish_analytics::{CloudCostCalculator, EstimateInput, PricingCatalog, Workload};

use crate::args::CostEstimateArgs;

fn resolve_workload(args: &CostEstimateArgs) -> Result<Workload, String> {
    if let Some(spec) = &args.durations {
        return Workload::parse_inline(spec);
    }
    if let Some(path) = &args.tasks_json {
        return Workload::load_json(path);
    }
    Err(
        "no workload provided; pass --durations \"label=seconds,...\" or \
         --tasks-json <file>"
            .to_string(),
    )
}

pub fn run_cost_estimate(args: CostEstimateArgs) -> std::process::ExitCode {
    println!("=== Fish Cloud Cost Calculator ===");

    let catalog = match &args.pricing_file {
        Some(path) => match PricingCatalog::load(path) {
            Ok(catalog) => catalog,
            Err(err) => {
                eprintln!("error: {err}");
                return std::process::ExitCode::FAILURE;
            }
        },
        None => PricingCatalog::default(),
    };

    let workload = match resolve_workload(&args) {
        Ok(workload) => workload,
        Err(err) => {
            eprintln!("error: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if workload.tasks.is_empty() {
        eprintln!("error: workload contains no tasks");
        return std::process::ExitCode::FAILURE;
    }

    let mut input = EstimateInput {
        parallelism: args.parallelism.max(1),
        use_spot: false,
        instance_name: args.instance.clone(),
        artifact_egress_gb: args.egress_gb.max(0.0),
        cache_storage_gb: args.storage_gb.max(0.0),
        retention_months: args.retention_months.max(1),
        cached_task_labels: args.cached.iter().cloned().collect(),
    };

    if args.parallelism == 0 {
        input.parallelism = 1;
    }

    let calculator = CloudCostCalculator::new(&catalog);

    let providers: Vec<String> = match &args.providers {
        Some(list) => list
            .split(',')
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .collect(),
        None => catalog.providers.keys().cloned().collect(),
    };

    for provider in &providers {
        if !catalog.providers.contains_key(provider) {
            eprintln!("error: unknown provider `{provider}` in this pricing catalog");
            return std::process::ExitCode::FAILURE;
        }
    }

    let report = match calculator.report(&workload, &input, None) {
        Ok(mut report) => {
            report.estimates.retain(|name, _| providers.contains(name));
            match report.estimates.values().next() {
                Some(_) => report,
                None => {
                    eprintln!("error: no provider estimates were produced");
                    return std::process::ExitCode::FAILURE;
                }
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    if args.json {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("error: serializing report failed: {err}");
                return std::process::ExitCode::FAILURE;
            }
        }
        return std::process::ExitCode::SUCCESS;
    }

    render_report(&report);
    std::process::ExitCode::SUCCESS
}

fn render_report(report: &fish_analytics::SavingsReport) {
    println!(
        "Catalog `{}` (prices as of {})",
        report.catalog_version, report.prices_as_of
    );
    println!(
        "Workload: {} tasks, {:.1}s total CPU-time",
        report.workload_tasks, report.workload_cpu_secs
    );
    if report.cached_skipped > 0 {
        println!(
            "Cache hits skipped from estimate: {}",
            report.cached_skipped
        );
    }
    println!();

    let mut providers: Vec<(&String, &fish_analytics::ProviderEstimates)> =
        report.estimates.iter().collect();
    providers.sort_by(|a, b| {
        a.1.spot
            .total_cost_usd
            .partial_cmp(&b.1.spot.total_cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (provider, estimates) in providers {
        println!(
            "▶ {provider} — {} x {} ({} jobs)",
            estimates.spot.fleet_size, estimates.spot.instance_name, estimates.spot.parallelism
        );
        println!(
            "  wall clock      : {:.1}s ({:.3} billable hours)",
            estimates.ondemand.wall_clock_secs, estimates.ondemand.billable_hours
        );
        println!(
            "  on-demand total : ${:.4}  (compute ${:.4}, egress ${:.4}, storage ${:.4})",
            estimates.ondemand.total_cost_usd,
            estimates.ondemand.compute_cost_usd,
            estimates.ondemand.egress_cost_usd,
            estimates.ondemand.storage_cost_usd
        );
        println!(
            "  spot total      : ${:.4}  (saves ${:.4} / {:.1}%)",
            estimates.spot.total_cost_usd, estimates.spot_savings_usd, estimates.spot_savings_pct
        );
        println!();
    }

    println!(
        "★ Recommendation: `{}` on `{}` at ${:.4}/run",
        report.recommended_provider, report.recommended_instance, report.recommended_spot_cost_usd
    );
}

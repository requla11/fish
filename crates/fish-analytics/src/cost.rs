use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{BuildMetrics, CacheMetrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancePrice {
    pub name: String,
    pub vcpus: u32,
    pub memory_gb: f64,
    pub ondemand_hourly_usd: f64,
    pub spot_hourly_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPricing {
    pub storage_per_gb_month: f64,
    pub egress_per_gb: f64,
    pub instances: Vec<InstancePrice>,
}

impl ProviderPricing {
    pub fn instance(&self, name: &str) -> Option<&InstancePrice> {
        self.instances.iter().find(|i| i.name == name)
    }

    pub fn cheapest_instance(&self) -> Option<&InstancePrice> {
        self.instances.iter().min_by(|a, b| {
            a.ondemand_hourly_usd
                .partial_cmp(&b.ondemand_hourly_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

/// A snapshot of public list prices for the embedded defaults. The numbers
/// are approximations intended for relative comparisons only; production
/// users should load an up-to-date catalog with [`PricingCatalog::load`].
pub const DEFAULT_CATALOG_VERSION: &str = "embedded-defaults";
pub const DEFAULT_PRICES_AS_OF: &str = "2026-01";

fn instance(name: &str, vcpus: u32, memory_gb: f64, od: f64, spot: f64) -> InstancePrice {
    InstancePrice {
        name: name.to_string(),
        vcpus,
        memory_gb,
        ondemand_hourly_usd: od,
        spot_hourly_usd: spot,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingCatalog {
    pub catalog_version: String,
    pub prices_as_of: String,
    pub providers: BTreeMap<String, ProviderPricing>,
}

impl Default for PricingCatalog {
    fn default() -> Self {
        let mut providers = BTreeMap::new();

        providers.insert(
            "aws".to_string(),
            ProviderPricing {
                storage_per_gb_month: 0.023,
                egress_per_gb: 0.09,
                instances: vec![
                    instance("t3.medium", 2, 4.0, 0.0416, 0.0125),
                    instance("c7i.xlarge", 4, 8.0, 0.1785, 0.0624),
                    instance("m7i.xlarge", 4, 16.0, 0.2016, 0.0706),
                    instance("c7i.2xlarge", 8, 16.0, 0.357, 0.1249),
                    instance("c7i.4xlarge", 16, 32.0, 0.714, 0.2499),
                ],
            },
        );

        providers.insert(
            "gcp".to_string(),
            ProviderPricing {
                storage_per_gb_month: 0.02,
                egress_per_gb: 0.12,
                instances: vec![
                    instance("e2-medium", 2, 4.0, 0.0335, 0.0101),
                    instance("e2-standard-4", 4, 16.0, 0.134, 0.0402),
                    instance("n2-standard-4", 4, 16.0, 0.1941, 0.0582),
                    instance("n2-standard-8", 8, 32.0, 0.3882, 0.1165),
                ],
            },
        );

        providers.insert(
            "azure".to_string(),
            ProviderPricing {
                storage_per_gb_month: 0.018,
                egress_per_gb: 0.087,
                instances: vec![
                    instance("B2s", 2, 4.0, 0.0416, 0.0125),
                    instance("D4s_v5", 4, 16.0, 0.192, 0.0576),
                    instance("D8s_v5", 8, 32.0, 0.384, 0.1152),
                    instance("F16s_v2", 16, 32.0, 0.752, 0.2256),
                ],
            },
        );

        Self {
            catalog_version: DEFAULT_CATALOG_VERSION.to_string(),
            prices_as_of: DEFAULT_PRICES_AS_OF.to_string(),
            providers,
        }
    }
}

impl PricingCatalog {
    /// Load a TOML pricing catalog, overriding the embedded defaults.
    ///
    /// Expected shape:
    /// ```toml
    /// catalog_version = "my-org-2026-q3"
    /// prices_as_of = "2026-07"
    /// [providers.aws]
    /// storage_per_gb_month = 0.023
    /// egress_per_gb = 0.09
    /// [[providers.aws.instances]]
    /// name = "c7i.xlarge"
    /// vcpus = 4
    /// memory_gb = 8.0
    /// ondemand_hourly_usd = 0.1785
    /// spot_hourly_usd = 0.0624
    /// ```
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read pricing catalog {}: {e}", path.display()))?;
        let mut catalog: PricingCatalog = toml::from_str(&content)
            .map_err(|e| format!("invalid pricing catalog {}: {e}", path.display()))?;
        if catalog.catalog_version.is_empty() {
            catalog.catalog_version = path.display().to_string();
        }
        Ok(catalog)
    }

    pub fn provider(&self, name: &str) -> Result<&ProviderPricing, String> {
        self.providers.get(name).ok_or_else(|| {
            format!(
                "unknown provider `{name}` in catalog `{}` (available: {})",
                self.catalog_version,
                self.providers
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDuration {
    pub label: String,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workload {
    pub tasks: Vec<TaskDuration>,
}

impl Workload {
    pub fn from_durations(pairs: impl IntoIterator<Item = (String, f64)>) -> Self {
        Self {
            tasks: pairs
                .into_iter()
                .map(|(label, duration_secs)| TaskDuration {
                    label,
                    duration_secs,
                })
                .collect(),
        }
    }

    pub fn parse_inline(spec: &str) -> Result<Self, String> {
        let mut tasks = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (label, secs) = part.split_once('=').ok_or_else(|| {
                format!("invalid duration entry `{part}`; expected `label=seconds`")
            })?;
            let label = label.trim();
            if label.is_empty() {
                return Err(format!("invalid duration entry `{part}`; label is empty"));
            }
            let duration_secs: f64 = secs
                .trim()
                .parse()
                .map_err(|_| format!("invalid seconds value in `{part}`"))?;
            if !(duration_secs.is_finite()) || duration_secs < 0.0 {
                return Err(format!(
                    "duration must be a non-negative number in `{part}`"
                ));
            }
            tasks.push(TaskDuration {
                label: label.to_string(),
                duration_secs,
            });
        }
        Ok(Self { tasks })
    }

    pub fn load_json(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read workload {}: {e}", path.display()))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("invalid workload JSON {}: {e}", path.display()))
    }

    /// Total serial CPU-time of all tasks.
    pub fn total_cpu_seconds(&self) -> f64 {
        self.tasks.iter().map(|t| t.duration_secs).sum()
    }

    /// Longest single task; lower-bounds any schedule's wall clock.
    pub fn critical_task_seconds(&self) -> f64 {
        self.tasks
            .iter()
            .map(|t| t.duration_secs)
            .fold(0.0_f64, f64::max)
    }

    /// Greedy LPT (longest processing time first) packing onto `slots`.
    /// Returns `(wall_clock_seconds, per_slot_loads)` sorted descending by
    /// load. With `slots == 0` this degenerates to serial execution time.
    pub fn lpt_schedule(&self, slots: usize) -> (f64, Vec<f64>) {
        let slots = slots.max(1);
        let mut loads = vec![0.0_f64; slots];
        let mut durations: Vec<f64> = self.tasks.iter().map(|t| t.duration_secs).collect();
        durations.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        for dur in durations {
            if let Some(slot) = loads
                .iter_mut()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx)
            {
                loads[slot] += dur;
            }
        }
        loads.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let wall = loads.first().copied().unwrap_or(0.0);
        (wall, loads)
    }

    /// Copy of the workload excluding cached tasks.
    pub fn without_cached(&self, cached: &HashSet<String>) -> Self {
        Self {
            tasks: self
                .tasks
                .iter()
                .filter(|t| !cached.contains(&t.label))
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateInput {
    pub parallelism: usize,
    pub use_spot: bool,
    pub instance_name: Option<String>,
    pub artifact_egress_gb: f64,
    pub cache_storage_gb: f64,
    pub retention_months: u32,
    pub cached_task_labels: HashSet<String>,
}

impl Default for EstimateInput {
    fn default() -> Self {
        Self {
            parallelism: 8,
            use_spot: false,
            instance_name: None,
            artifact_egress_gb: 0.0,
            cache_storage_gb: 0.0,
            retention_months: 1,
            cached_task_labels: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub provider: String,
    pub instance_name: String,
    pub fleet_size: usize,
    pub parallelism: usize,
    pub scheduled_tasks: usize,
    pub skipped_cached_tasks: usize,
    pub total_task_cpu_secs: f64,
    pub wall_clock_secs: f64,
    pub billable_hours: f64,
    pub hourly_rate_usd: f64,
    pub pricing_mode: String,
    pub compute_cost_usd: f64,
    pub egress_cost_usd: f64,
    pub storage_cost_usd: f64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEstimates {
    pub ondemand: CostEstimate,
    pub spot: CostEstimate,
    pub spot_savings_usd: f64,
    pub spot_savings_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsReport {
    pub catalog_version: String,
    pub prices_as_of: String,
    pub workload_tasks: usize,
    pub workload_cpu_secs: f64,
    pub cached_skipped: usize,
    pub estimates: BTreeMap<String, ProviderEstimates>,
    pub recommended_provider: String,
    pub recommended_instance: String,
    pub recommended_spot_cost_usd: f64,
    pub local_baseline_secs: Option<f64>,
}

pub struct CloudCostCalculator<'a> {
    catalog: &'a PricingCatalog,
}

impl<'a> CloudCostCalculator<'a> {
    pub fn new(catalog: &'a PricingCatalog) -> Self {
        Self { catalog }
    }

    /// Build a workload from recorded build metrics: every built package is
    /// charged its measured duration, cached packages are treated as hits.
    pub fn workload_from_metrics(&self, build: &BuildMetrics, _cache: &CacheMetrics) -> Workload {
        let per_task = build.build_duration_secs / build.packages_built.max(1) as f64;
        Workload::from_durations(
            (0..build.packages_built).map(|i| (format!("package_{i}"), per_task)),
        )
    }

    /// Price one provider for the given workload under both on-demand and
    /// spot rates.
    pub fn estimate_provider(
        &self,
        provider_name: &str,
        workload: &Workload,
        input: &EstimateInput,
    ) -> Result<ProviderEstimates, String> {
        let pricing = self.catalog.provider(provider_name)?;

        let instance = match &input.instance_name {
            Some(name) => pricing.instance(name).ok_or_else(|| {
                format!(
                    "provider `{provider_name}` has no instance `{name}` (available: {})",
                    pricing
                        .instances
                        .iter()
                        .map(|i| i.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?,
            None => pricing
                .cheapest_instance()
                .ok_or_else(|| format!("provider `{provider_name}` lists no instances"))?,
        };

        let effective = workload.without_cached(&input.cached_task_labels);
        let skipped = workload.tasks.len() - effective.tasks.len();

        // One VM can host `instance.vcpus` build jobs; the fleet must cover
        // the requested parallelism.
        let fleet_size = (input.parallelism.div_ceil(instance.vcpus.max(1) as usize)).max(1);
        let total_slots = fleet_size * instance.vcpus.max(1) as usize;

        let (wall, _) = effective.lpt_schedule(total_slots);

        let ondemand = self.price_estimate(
            provider_name,
            instance,
            fleet_size,
            input.parallelism,
            &effective,
            skipped,
            wall,
            false,
            input,
        );
        let spot = self.price_estimate(
            provider_name,
            instance,
            fleet_size,
            input.parallelism,
            &effective,
            skipped,
            wall,
            true,
            input,
        );

        let spot_savings_usd = (ondemand.total_cost_usd - spot.total_cost_usd).max(0.0);
        let spot_savings_pct = if ondemand.total_cost_usd > 0.0 {
            spot_savings_usd / ondemand.total_cost_usd * 100.0
        } else {
            0.0
        };

        Ok(ProviderEstimates {
            ondemand,
            spot,
            spot_savings_usd,
            spot_savings_pct,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn price_estimate(
        &self,
        provider_name: &str,
        instance: &InstancePrice,
        fleet_size: usize,
        parallelism: usize,
        effective: &Workload,
        skipped: usize,
        wall_secs: f64,
        use_spot: bool,
        input: &EstimateInput,
    ) -> CostEstimate {
        let pricing = &self.catalog.providers[provider_name];
        let hourly_rate = if use_spot {
            instance.spot_hourly_usd
        } else {
            instance.ondemand_hourly_usd
        };
        let billable_hours = wall_secs / 3600.0;
        let compute = billable_hours * hourly_rate * fleet_size as f64;
        let egress = input.artifact_egress_gb * pricing.egress_per_gb;
        let storage = input.cache_storage_gb
            * pricing.storage_per_gb_month
            * input.retention_months.max(1) as f64;

        CostEstimate {
            provider: provider_name.to_string(),
            instance_name: instance.name.clone(),
            fleet_size,
            parallelism,
            scheduled_tasks: effective.tasks.len(),
            skipped_cached_tasks: skipped,
            total_task_cpu_secs: effective.total_cpu_seconds(),
            wall_clock_secs: wall_secs,
            billable_hours,
            hourly_rate_usd: hourly_rate,
            pricing_mode: if use_spot {
                "spot".to_string()
            } else {
                "ondemand".to_string()
            },
            compute_cost_usd: compute,
            egress_cost_usd: egress,
            storage_cost_usd: storage,
            total_cost_usd: compute + egress + storage,
        }
    }

    /// Full report across every provider in the catalog, ranking them by
    /// monthly spot cost for the requested workload.
    pub fn report(
        &self,
        workload: &Workload,
        input: &EstimateInput,
        local_baseline_secs: Option<f64>,
    ) -> Result<SavingsReport, String> {
        let mut estimates = BTreeMap::new();
        for provider_name in self.catalog.providers.keys() {
            let est = self.estimate_provider(provider_name, workload, input)?;
            estimates.insert(provider_name.clone(), est);
        }

        let (best_provider, recommended_instance, recommended_spot_cost_usd) = {
            let mut ranked: Vec<(&String, &ProviderEstimates)> = estimates.iter().collect();
            ranked.sort_by(|a, b| {
                a.1.spot
                    .total_cost_usd
                    .partial_cmp(&b.1.spot.total_cost_usd)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let (best_provider, best_est) = ranked
                .first()
                .ok_or_else(|| "pricing catalog contains no providers".to_string())?;
            (
                (*best_provider).clone(),
                best_est.spot.instance_name.clone(),
                best_est.spot.total_cost_usd,
            )
        };

        Ok(SavingsReport {
            catalog_version: self.catalog.catalog_version.clone(),
            prices_as_of: self.catalog.prices_as_of.clone(),
            workload_tasks: workload.tasks.len(),
            workload_cpu_secs: workload.total_cpu_seconds(),
            cached_skipped: workload.tasks.len()
                - workload
                    .without_cached(&input.cached_task_labels)
                    .tasks
                    .len(),
            estimates,
            recommended_provider: best_provider,
            recommended_instance,
            recommended_spot_cost_usd,
            local_baseline_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workload(pairs: &[(&str, f64)]) -> Workload {
        Workload::from_durations(pairs.iter().map(|(l, d)| ((*l).to_string(), *d)))
    }

    #[test]
    fn test_lpt_schedule_balances_known_loads() {
        // 4 tasks of 100s each on 2 slots must land at exactly 200s.
        let w = workload(&[("a", 100.0), ("b", 100.0), ("c", 100.0), ("d", 100.0)]);
        let (wall, loads) = w.lpt_schedule(2);
        assert_eq!(wall, 200.0);
        assert_eq!(loads.len(), 2);
        assert!(loads.iter().all(|l| (l - 200.0).abs() < 1e-9));

        // One slot degenerates to serial time.
        let (serial_wall, _) = w.lpt_schedule(1);
        assert_eq!(serial_wall, 400.0);
    }

    #[test]
    fn test_lpt_longest_first_prevents_straggler() {
        // The 50s job lower-bounds any schedule at 50s; LPT reaches that
        // bound by stacking both short jobs behind it.
        let w = workload(&[("big", 50.0), ("s1", 10.0), ("s2", 10.0)]);
        let (wall, _) = w.lpt_schedule(2);
        assert_eq!(wall, 50.0);
    }

    #[test]
    fn test_workload_parse_inline() {
        let w = Workload::parse_inline("core=12.5, cli=3, =bad").unwrap_err();
        assert!(w.contains("invalid duration entry"));

        let good = Workload::parse_inline("core=12.5, cli=3").unwrap();
        assert_eq!(good.tasks.len(), 2);
        assert!((good.total_cpu_seconds() - 15.5).abs() < 1e-9);

        let negative = Workload::parse_inline("x=-4");
        assert!(negative.is_err());
    }

    #[test]
    fn test_estimate_provider_math_is_exact() {
        let catalog = PricingCatalog::default();
        let calc = CloudCostCalculator::new(&catalog);
        let w = workload(&[
            ("t1", 3600.0),
            ("t2", 1800.0),
            ("t3", 1800.0),
            ("t4", 900.0),
        ]);

        let input = EstimateInput {
            parallelism: 8,
            use_spot: false,
            instance_name: Some("c7i.2xlarge".to_string()),
            artifact_egress_gb: 0.0,
            cache_storage_gb: 0.0,
            retention_months: 1,
            cached_task_labels: HashSet::new(),
        };

        let est = calc.estimate_provider("aws", &w, &input).unwrap();

        // 8 vCPUs across one c7i.2xlarge -> single fleet node.
        assert_eq!(est.ondemand.fleet_size, 1);
        assert_eq!(est.ondemand.scheduled_tasks, 4);
        // LPT: slots [3600+900=4500? no: 8 slots] -> wall == critical task.
        let (expected_wall, _) = w.lpt_schedule(8);
        assert_eq!(est.ondemand.wall_clock_secs, expected_wall);
        assert!((est.ondemand.wall_clock_secs - 3600.0).abs() < 1e-9);
        assert!((est.ondemand.billable_hours - 1.0).abs() < 1e-9);
        assert!((est.ondemand.compute_cost_usd - 0.357).abs() < 1e-9);
        assert!((est.ondemand.total_cost_usd - est.ondemand.compute_cost_usd).abs() < 1e-9);

        // Spot must be strictly cheaper with identical scheduling.
        assert!(est.spot.total_cost_usd < est.ondemand.total_cost_usd);
        assert!(est.spot_savings_pct > 50.0);
    }

    #[test]
    fn test_cache_hits_reduce_cost() {
        let catalog = PricingCatalog::default();
        let calc = CloudCostCalculator::new(&catalog);
        let w = workload(&[("a", 600.0), ("b", 600.0), ("c_hit", 600.0)]);

        let input = EstimateInput {
            parallelism: 16,
            instance_name: Some("c7i.xlarge".to_string()),
            cached_task_labels: HashSet::from(["c_hit".to_string()]),
            ..EstimateInput::default()
        };

        let est = calc.estimate_provider("aws", &w, &input).unwrap();
        assert_eq!(est.ondemand.scheduled_tasks, 2);
        assert_eq!(est.ondemand.skipped_cached_tasks, 1);
        assert!(est.ondemand.compute_cost_usd > 0.0);
    }

    #[test]
    fn test_egress_and_storage_costs_flow_into_total() {
        let catalog = PricingCatalog::default();
        let calc = CloudCostCalculator::new(&catalog);
        let w = workload(&[("a", 60.0)]);

        let input = EstimateInput {
            parallelism: 4,
            instance_name: Some("c7i.xlarge".to_string()),
            artifact_egress_gb: 10.0,
            cache_storage_gb: 100.0,
            retention_months: 3,
            ..EstimateInput::default()
        };

        let est = calc.estimate_provider("aws", &w, &input).unwrap();
        assert!((est.ondemand.egress_cost_usd - 0.90).abs() < 1e-6);
        assert!(
            (est.ondemand.storage_cost_usd - (100.0 * 0.023 * 3.0)).abs() < 1e-6,
            "storage = gb * price * months"
        );
        let parts = est.ondemand.compute_cost_usd
            + est.ondemand.egress_cost_usd
            + est.ondemand.storage_cost_usd;
        assert!((est.ondemand.total_cost_usd - parts).abs() < 1e-9);
    }

    #[test]
    fn test_unknown_provider_and_instance_fail_loudly() {
        let catalog = PricingCatalog::default();
        let calc = CloudCostCalculator::new(&catalog);
        let w = workload(&[("a", 1.0)]);

        let err = calc
            .estimate_provider("alibaba", &w, &EstimateInput::default())
            .unwrap_err();
        assert!(err.contains("unknown provider"));

        let input = EstimateInput {
            instance_name: Some("supercomputer-9k".to_string()),
            ..EstimateInput::default()
        };
        let err = calc.estimate_provider("aws", &w, &input).unwrap_err();
        assert!(err.contains("no instance"));
    }

    #[test]
    fn test_report_ranks_providers_and_serializes() {
        let catalog = PricingCatalog::default();
        let calc = CloudCostCalculator::new(&catalog);
        let w = workload(&[("a", 300.0), ("b", 120.0)]);

        let report = calc
            .report(&w, &EstimateInput::default(), Some(420.0))
            .unwrap();
        assert_eq!(report.estimates.len(), 3);
        assert!(!report.recommended_provider.is_empty());
        assert!(report.workload_cpu_secs > 0.0);

        let json = serde_json::to_string_pretty(&report).unwrap();
        let parsed: SavingsReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.recommended_provider, report.recommended_provider);
        assert!(json.contains("prices_as_of"));
    }

    #[test]
    fn test_custom_catalog_roundtrip_through_toml() {
        let toml_catalog = r#"
catalog_version = "org-custom"
prices_as_of = "2026-08"

[providers.onprem-k8s]
storage_per_gb_month = 0.004
egress_per_gb = 0.0

[[providers.onprem-k8s.instances]]
name = "build-node-32c"
vcpus = 32
memory_gb = 64.0
ondemand_hourly_usd = 0.42
spot_hourly_usd = 0.42
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pricing.toml");
        std::fs::write(&path, toml_catalog).unwrap();

        let catalog = PricingCatalog::load(&path).unwrap();
        assert_eq!(catalog.catalog_version, "org-custom");
        assert_eq!(catalog.providers.len(), 1);

        let calc = CloudCostCalculator::new(&catalog);
        let w = workload(&[("a", 1000.0)]);
        let est = calc
            .estimate_provider("onprem-k8s", &w, &EstimateInput::default())
            .unwrap();
        // 8 parallelism over a 32-core node -> still one node.
        assert_eq!(est.ondemand.fleet_size, 1);
        assert!(est.ondemand.total_cost_usd > 0.0);
    }

    #[test]
    fn test_workload_from_metrics_splits_evenly() {
        let catalog = PricingCatalog::default();
        let calc = CloudCostCalculator::new(&catalog);
        let build = BuildMetrics {
            build_duration_secs: 100.0,
            cache_saved_time_secs: 0.0,
            packages_built: 4,
            packages_cached: 0,
            timestamp: chrono::Utc::now(),
        };
        let cache = CacheMetrics {
            total_hits: 0,
            total_misses: 0,
            total_requests: 0,
            hit_rate: 0.0,
            cache_size_bytes: 0,
            timestamp: chrono::Utc::now(),
        };
        let w = calc.workload_from_metrics(&build, &cache);
        assert_eq!(w.tasks.len(), 4);
        assert!((w.total_cpu_seconds() - 100.0).abs() < 1e-9);
    }
}

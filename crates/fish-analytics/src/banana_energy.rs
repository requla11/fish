use banana::telemetry::{EnergyMeter, EnergyMetrics, HardwareProfile};

pub struct FishEnergyTracker {
    meter: EnergyMeter,
}

impl FishEnergyTracker {
    pub fn new(tdp_watts: f64, grid_intensity_g_per_kwh: f64) -> Self {
        let profile = HardwareProfile {
            tdp_watts,
            idle_power_watts: 10.0,
            core_count: std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4),
        };
        Self {
            meter: EnergyMeter::new(profile, grid_intensity_g_per_kwh),
        }
    }

    pub fn start_session(&mut self) {
        self.meter.start();
    }

    pub fn end_session(&self, cpu_utilization: f64) -> EnergyMetrics {
        self.meter.stop(cpu_utilization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fish_energy_tracker_integration() {
        let mut tracker = FishEnergyTracker::new(95.0, 250.0);
        tracker.start_session();
        let metrics = tracker.end_session(0.6);
        assert!(metrics.cpu_cores_utilized > 0.0);
    }
}

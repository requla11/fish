from typing import List, Dict, Any

class PredictivePrRiskScorer:
    CRITICAL_PATTERNS = [
        "Cargo.lock",
        "Cargo.toml",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "go.mod",
        "go.sum",
        "fish.toml",
        "fish.yaml",
        "CMakeLists.txt",
        "Makefile",
        "Dockerfile",
        ".github/workflows",
    ]

    CORE_COMPONENTS = [
        "crates/fish-core",
        "crates/fish-graph",
        "crates/fish-scheduler",
        "crates/fish-executor",
        "crates/fish-cache",
        "crates/fish-cas",
    ]

    def __init__(self):
        pass

    def compute_pr_risk(self, changed_files: List[str], lines_added: int, lines_deleted: int) -> Dict[str, Any]:
        risk_score = 0.0
        risk_factors = []

        for f in changed_files:
            for crit in self.CRITICAL_PATTERNS:
                if f.endswith(crit) or crit in f:
                    risk_score += 0.30
                    risk_factors.append(f"Critical build or manifest configuration modified: {f}")
                    break

            for core in self.CORE_COMPONENTS:
                if core in f:
                    risk_score += 0.20
                    risk_factors.append(f"Core orchestration engine path modified: {f}")
                    break

        total_churn = lines_added + lines_deleted
        if total_churn > 1500:
            risk_score += 0.35
            risk_factors.append(f"Very high code churn ({total_churn} lines)")
        elif total_churn > 500:
            risk_score += 0.20
            risk_factors.append(f"Moderate code churn ({total_churn} lines)")

        if len(changed_files) > 20:
            risk_score += 0.25
            risk_factors.append(f"Broad blast radius across {len(changed_files)} packages/files")
        elif len(changed_files) > 8:
            risk_score += 0.15

        risk_score = min(1.0, max(0.0, risk_score))

        if risk_score > 0.65:
            severity = "HIGH"
            suggested_action = "Execute full workspace regression tests and require multi-reviewer sign-off."
        elif risk_score > 0.35:
            severity = "MEDIUM"
            suggested_action = "Execute targeted affected-package dependency tests."
        else:
            severity = "LOW"
            suggested_action = "Standard fast CI validation."

        return {
            "risk_score": round(risk_score, 2),
            "severity": severity,
            "risk_factors": list(dict.fromkeys(risk_factors)),
            "suggested_action": suggested_action
        }

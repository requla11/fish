from pathlib import PurePosixPath
from typing import List, Dict, Any


class SemanticImpactAnalyzer:
    """Path-convention based impact analysis.

    Without a persisted symbol/call-graph index this analyzer cannot resolve
    symbol-level reachability. It does compute a *real* inference from file
    paths: in a Cargo workspace layout (`crates/<name>/...`) each modified
    file maps deterministically to its owning crate target. Symbol deltas are
    reported back untouched with ``unresolved_symbols`` so callers know they
    were not analyzed.
    """

    def __init__(self):
        pass

    def analyze_semantic_impact(self, modified_files: List[str], symbol_deltas: List[str]) -> Dict[str, Any]:
        impacted_targets = sorted(
            {
                self._crate_target_for_file(f)
                for f in modified_files
                if self._crate_target_for_file(f) is not None
            }
        )

        if not modified_files and not symbol_deltas:
            status = "NO_INPUT"
        elif impacted_targets:
            status = "PATH_HEURISTIC"
        else:
            status = "UNRESOLVED"

        return {
            "status": status,
            "modified_files": list(modified_files),
            "impacted_targets": impacted_targets,
            "unresolved_symbols": list(symbol_deltas),
            "analysis": (
                "Targets inferred from workspace path conventions only; "
                "symbol-level impact requires a semantic index that fish does "
                "not build yet."
            ),
        }

    @staticmethod
    def _crate_target_for_file(file_path: str) -> str | None:
        parts = PurePosixPath(file_path.replace("\\", "/")).parts
        if len(parts) >= 2 and parts[0] == "crates":
            return parts[1]
        if len(parts) >= 2 and parts[0] == "src":
            return None
        return None

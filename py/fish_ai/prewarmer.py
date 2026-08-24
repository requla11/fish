from typing import List, Dict, Any


class PredictiveBuildPrewarmer:
    """Keystroke-driven prewarm *suggestions*.

    The extension-to-checker mapping below is a real heuristic about which
    toolchain a file edit would exercise. Fish does not wire these suggestions
    into any scheduler or background executor yet, so the response says
    ``SUGGESTIONS_ONLY`` instead of claiming work was scheduled.
    """

    def __init__(self):
        self.typing_history: List[str] = []

    def on_keystroke_activity(self, active_file: str, dirty_symbol: str) -> Dict[str, Any]:
        self.typing_history.append(active_file)
        if len(self.typing_history) > 20:
            self.typing_history.pop(0)

        predicted_targets = []
        if active_file.endswith(".rs"):
            predicted_targets.append("cargo-check")
        elif active_file.endswith(".ts") or active_file.endswith(".js"):
            predicted_targets.append("tsc-check")
        elif active_file.endswith(".go"):
            predicted_targets.append("go-vet")

        return {
            "status": "SUGGESTIONS_ONLY",
            "active_file": active_file,
            "dirty_symbol": dirty_symbol,
            "prewarm_targets": predicted_targets,
            "note": (
                "Targets are heuristic suggestions; no scheduler or executor "
                "is attached yet."
            ),
        }

from typing import Dict, Any

class AiAutoFixer:
    """Single-pattern import fixer.

    Handles exactly one failure class: a missing `use` declaration for
    PathBuf/HashMap reported through "cannot find"/E0432-style messages.
    Anything else returns NO_FIX_IDENTIFIED instead of pretending a fix
    exists.
    """

    def __init__(self):
        pass

    def generate_fix(self, file_content: str, error_message: str) -> Dict[str, Any]:
        remediated_content = file_content
        identified_import: str | None = None

        if "cannot find" in error_message or "E0433" in error_message:
            if "PathBuf" in error_message and "use std::path::PathBuf;" not in file_content:
                identified_import = "std::path::PathBuf"
                remediated_content = f"use std::path::PathBuf;\n{file_content}"
            elif (
                "HashMap" in error_message
                and "use std::collections::HashMap;" not in file_content
            ):
                identified_import = "std::collections::HashMap"
                remediated_content = f"use std::collections::HashMap;\n{file_content}"

        if identified_import is not None:
            return {
                "status": "FIX_PROPOSED",
                "modified": True,
                "patched_source": remediated_content,
                "explanation": (
                    f"Prepended `use {identified_import};` because the compiler "
                    "message references the type without an import."
                ),
            }

        return {
            "status": "NO_FIX_IDENTIFIED",
            "modified": False,
            "patched_source": file_content,
            "explanation": (
                "This fixer only recognizes missing PathBuf/HashMap imports; "
                "the provided error did not match that pattern."
            ),
        }


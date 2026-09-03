from typing import Dict, Any, Optional

class AiAutoFixer:
    RUST_TYPE_IMPORTS = {
        "PathBuf": "std::path::PathBuf",
        "Path": "std::path::Path",
        "HashMap": "std::collections::HashMap",
        "HashSet": "std::collections::HashSet",
        "BTreeMap": "std::collections::BTreeMap",
        "BTreeSet": "std::collections::BTreeSet",
        "Arc": "std::sync::Arc",
        "Mutex": "std::sync::Mutex",
        "RwLock": "std::sync::RwLock",
        "Duration": "std::time::Duration",
        "Instant": "std::time::Instant",
    }

    TS_HOOK_IMPORTS = {
        "useState": "useState",
        "useEffect": "useEffect",
        "useMemo": "useMemo",
        "useCallback": "useCallback",
        "useRef": "useRef",
    }

    def __init__(self):
        pass

    def generate_fix(self, file_content: str, error_message: str) -> Dict[str, Any]:
        remediated_content = file_content
        identified_import: Optional[str] = None

        if "cannot find" in error_message or "E0433" in error_message or "E0412" in error_message:
            for type_name, full_path in self.RUST_TYPE_IMPORTS.items():
                if type_name in error_message:
                    import_stmt = f"use {full_path};"
                    if import_stmt not in file_content:
                        identified_import = full_path
                        remediated_content = f"{import_stmt}\n{file_content}"
                        break

        elif "Cannot find name" in error_message:
            for hook_name in self.TS_HOOK_IMPORTS:
                if f"'{hook_name}'" in error_message or f'"{hook_name}"' in error_message or hook_name in error_message:
                    if "from 'react'" in file_content or 'from "react"' in file_content:
                        identified_import = f"react::{hook_name}"
                        remediated_content = file_content.replace(
                            "import React",
                            f"import React, {{ {hook_name} }}"
                        )
                    else:
                        identified_import = f"react::{hook_name}"
                        remediated_content = f"import {{ {hook_name} }} from 'react';\n{file_content}"
                    break

        if identified_import is not None:
            return {
                "status": "FIX_PROPOSED",
                "modified": True,
                "patched_source": remediated_content,
                "explanation": f"Added missing import declaration for `{identified_import}`.",
            }

        return {
            "status": "NO_FIX_IDENTIFIED",
            "modified": False,
            "patched_source": file_content,
            "explanation": "No deterministic remediation pattern matched the provided compiler diagnostics.",
        }

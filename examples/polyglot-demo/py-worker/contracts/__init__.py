"""Contract package: the single source of truth for polyglot-demo events.

Other services (go-service, rust-service, web) read the JSON files in this
directory directly. This module is the Python-side accessor.
"""

import json
from pathlib import Path

CONTRACTS_DIR = Path(__file__).resolve().parent


def load_schema() -> dict:
    with (CONTRACTS_DIR / "events.schema.json").open(encoding="utf-8") as fh:
        return json.load(fh)


def load_topics() -> list[str]:
    with (CONTRACTS_DIR / "topics.json").open(encoding="utf-8") as fh:
        return json.load(fh)["topics"]


def required_fields() -> list[str]:
    return list(load_schema()["required"])

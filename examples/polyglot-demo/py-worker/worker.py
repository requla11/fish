"""Contract-first event worker.

Consumes TaskEvent dicts and validates them against the schema owned by this
project (contracts/events.schema.json). Validation is deliberately hand-rolled
on the standard library only, so the demo runs without installing anything.
"""

from datetime import datetime, timezone
from pathlib import Path

from contracts import load_schema, load_topics, required_fields

SCHEMA_PATH = Path(__file__).resolve().parent / "contracts" / "events.schema.json"


class ContractViolation(Exception):
    """Raised when an event does not satisfy the shared contract."""


def validate_event(event: dict) -> None:
    """Validate `event` against contracts/events.schema.json.

    Implements the subset of JSON Schema draft-07 that the contract uses:
    type, required, enum, minLength, additionalProperties.
    """
    schema = load_schema()
    props = schema.get("properties", {})

    if not isinstance(event, dict):
        raise ContractViolation("event must be an object")

    for field in required_fields():
        if field not in event:
            raise ContractViolation(f"missing required field: {field}")

    for name, sub in event.items():
        if name not in props and not schema.get("additionalProperties", True):
            raise ContractViolation(f"unknown field: {name}")
        spec = props.get(name)
        if not spec:
            continue

        expected = spec.get("type")
        checks = {
            "string": lambda v: isinstance(v, str),
            "object": lambda v: isinstance(v, dict),
            "number": lambda v: isinstance(v, (int, float)) and not isinstance(v, bool),
            "boolean": lambda v: isinstance(v, bool),
        }
        if expected in checks and not checks[expected](sub):
            raise ContractViolation(f"field {name!r} must be of type {expected}")

        if "enum" in spec and sub not in spec["enum"]:
            raise ContractViolation(
                f"field {name!r} must be one of {spec['enum']}, got {sub!r}"
            )

        if "minLength" in spec and len(sub) < spec["minLength"]:
            raise ContractViolation(f"field {name!r} is shorter than {spec['minLength']}")


def make_event(topic: str, payload: dict | None = None) -> dict:
    if topic not in load_topics():
        raise ContractViolation(f"unknown topic: {topic}")
    return {
        "id": f"evt-{datetime.now(timezone.utc).timestamp():.0f}",
        "topic": topic,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "payload": payload or {},
    }


def process(events: list[dict]) -> tuple[list[dict], list[tuple[dict, str]]]:
    """Validate a batch; returns (accepted, [(event, reason), ...])."""
    accepted: list[dict] = []
    rejected: list[tuple[dict, str]] = []
    for event in events:
        try:
            validate_event(event)
            accepted.append(event)
        except ContractViolation as exc:
            rejected.append((event, str(exc)))
    return accepted, rejected


def main() -> None:
    print(f"py-worker using contract at {SCHEMA_PATH}")

    good = make_event("build.finished", {"duration_ms": 18400})
    bad_topic = {"id": "evt-2", "topic": "deploy.started", "created_at": "2026-08-24T00:00:00Z"}
    missing_field = {"id": "evt-3", "topic": "build.started"}

    accepted, rejected = process([good, bad_topic, missing_field])
    for event in accepted:
        print(f"  [ok] accepted: {event['topic']} ({event['id']})")
    for event, reason in rejected:
        print(f"  [rejected] {reason} ({event.get('id', '?')})")

    raise SystemExit(0 if len(accepted) == 1 and len(rejected) == 2 else 1)


if __name__ == "__main__":
    main()

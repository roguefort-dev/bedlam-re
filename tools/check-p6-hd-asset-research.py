#!/usr/bin/python3
"""Validate the committed HD asset pipeline research doc (the P6 opener).

Fail-closed checker for the p6-hd-asset-research required gate
(docs/required-gates.toml, the e0bc7fb scaffold pattern applied to a research
artifact). PLAN §6 names docs/RESEARCH-HD-ASSET-PIPELINE.md as its own
prerequisite ("exact package/model pins come from ..."). The doc grades
itself through structure this checker enforces:

  1. the four PLAN §6 workflow categories ((a) background outpainting /
     generative fill, (b) alpha-aware sprite/sprite-sheet upscale,
     (c) seamless tile/texture upscale, (d) portraits/UI art) each have a
     section, and the machine-readable pin registry (schema
     hd-asset-pins-v1, a fenced TOML block inside the doc) gives every
     category at least one PRIMARY model pin -- category (a) additionally a
     fallback;
  2. pin discipline: first-party https URLs, retrieval dates inside the
     verification window, a license on every model pin, VERIFIED licenses on
     primary model pins (an explicit "unverified" license may never be a
     primary), and a note on every deferred pin;
  3. the plan's boundary sentences ride verbatim in the doc (git never
     carries pixels, outputs without recorded provenance are excluded from
     shipping, the runtime falls back to the original asset, the engine
     renders all text/controls/click targets/gameplay information);
  4. cross-artifact safety with docs/required-gates.toml: the
     p6-hd-asset-research gate is defined, wired into the P6 phase list,
     runs this checker, and tracks the doc.

It reads ONLY committed docs -- no network, no game-data read, no writes,
PATH-free under the validator's bwrap.
"""

from __future__ import annotations

import argparse
import datetime
import re
import sys
import tomllib
from pathlib import Path
from urllib.parse import urlparse

DOC_RELATIVE = "docs/RESEARCH-HD-ASSET-PIPELINE.md"
MANIFEST_RELATIVE = "docs/required-gates.toml"
GATE_ID = "p6-hd-asset-research"
CHECKER_RELATIVE = "tools/check-p6-hd-asset-research.py"
PIN_SCHEMA = "hd-asset-pins-v1"

CATEGORIES = (
    "background-outpaint",
    "sprite-upscale",
    "tile-texture-upscale",
    "portrait-ui",
)
KINDS = ("tool", "workflow-template", "model")
ROLES = ("primary", "fallback", "bootstrap", "deferred")
KIND_ROLES = {
    "tool": ("primary", "fallback", "bootstrap"),
    "workflow-template": ("primary", "deferred"),
    "model": ("primary", "fallback", "deferred"),
}
PIN_REQUIRED_KEYS = ("id", "kind", "role", "version", "url", "retrieved")
PIN_KEYS = frozenset(PIN_REQUIRED_KEYS) | {
    "revision",
    "license",
    "sha256",
    "note",
    "categories",
}
FIRST_PARTY_HOSTS = frozenset(
    {"github.com", "raw.githubusercontent.com", "huggingface.co", "docs.comfy.org"}
)
RETRIEVED_EARLIEST = datetime.date(2026, 8, 1)
REQUIRED_SECTIONS = (
    "## 1. Constraints and recommendation",
    "## 5. The four workflow categories",
    "### 5.A (a) Background outpainting / generative fill (4:3 → 16:9/16:10)",
    "### 5.B (b) Alpha-aware sprite / sprite-sheet upscale",
    "### 5.C (c) Seamless tile / texture upscale",
    "### 5.D (d) Portraits / UI art",
    "## 6. Provenance + manifest schema",
    "## 7. Automated gate criteria design",
    "## 8. Runtime resolution seam sketch",
    "## 9. Isolated, hardware-profiled setup posture",
)
REQUIRED_SENTENCES = (
    # the unit's own bounds
    "RESEARCH ONLY",
    # the D21 git boundary, PLAN §6 verbatim
    "Git contains only workflow JSON, recipes, masks, model/tool/version"
    " hashes, seeds/prompts, manifests and provenance",
    "Generated images live in a user-selected external HD-pack directory,"
    " never in git",
    # the exclusion rule the §7 gates enforce
    "outputs without recorded provenance are excluded from shipping",
    # the §8 runtime seam
    "falls back to the original asset",
    "engine renders all text, controls, click targets and gameplay information",
    # the two schema ids the design defines
    "hd-pack-manifest-v1",
    "hd-asset-pins-v1",
)
TOML_BLOCK = re.compile(r"```toml\r?\n(.*?)\r?\n```", re.DOTALL)


class ResearchError(Exception):
    pass


def load_doc(root: Path) -> str:
    path = root / DOC_RELATIVE
    try:
        return path.read_bytes().decode("utf-8")
    except OSError as error:
        raise ResearchError(f"research doc is missing: {path}") from error
    except UnicodeError as error:
        raise ResearchError(f"research doc is not UTF-8: {error}") from error


def normalize_ws(text: str) -> str:
    """Collapse markdown line-wrapping so wrapped sentences still match."""
    return " ".join(text.split())


def check_sections_and_sentences(text: str) -> None:
    flat = normalize_ws(text)
    for header in REQUIRED_SECTIONS:
        if normalize_ws(header) not in flat:
            raise ResearchError(f"research doc is missing section: {header!r}")
    for sentence in REQUIRED_SENTENCES:
        if normalize_ws(sentence) not in flat:
            raise ResearchError(
                f"research doc is missing the required rule sentence: {sentence!r}"
            )


def extract_pin_registry(text: str) -> str:
    blocks = TOML_BLOCK.findall(text)
    registries = [
        block
        for block in blocks
        if block.lstrip().startswith(f'schema = "{PIN_SCHEMA}"')
    ]
    if not registries:
        raise ResearchError(
            f"research doc has no fenced toml pin registry with"
            f' schema = "{PIN_SCHEMA}"'
        )
    if len(registries) > 1:
        raise ResearchError(
            f"research doc carries {len(registries)} {PIN_SCHEMA} blocks (want 1)"
        )
    return registries[0]


def check_retrieved(value: object, identifier: str, today: datetime.date) -> None:
    if not isinstance(value, str):
        raise ResearchError(f"pin {identifier} retrieved must be a date string")
    try:
        parsed = datetime.date.fromisoformat(value)
    except ValueError as error:
        raise ResearchError(
            f"pin {identifier} retrieved is not an ISO date: {value!r}"
        ) from error
    if parsed < RETRIEVED_EARLIEST:
        raise ResearchError(
            f"pin {identifier} retrieved {value} predates the verification window"
            f" ({RETRIEVED_EARLIEST})"
        )
    if parsed > today:
        raise ResearchError(
            f"pin {identifier} retrieved {value} is in the future (today {today})"
        )


def check_url(value: object, identifier: str) -> None:
    if not isinstance(value, str):
        raise ResearchError(f"pin {identifier} url must be a string")
    parsed = urlparse(value)
    if parsed.scheme != "https" or parsed.hostname is None:
        raise ResearchError(f"pin {identifier} url must be https: {value!r}")
    if parsed.hostname not in FIRST_PARTY_HOSTS:
        raise ResearchError(
            f"pin {identifier} url host {parsed.hostname!r} is not a first-party"
            f" source ({sorted(FIRST_PARTY_HOSTS)})"
        )


def load_pins(text: str, today: datetime.date) -> list[dict]:
    registry = extract_pin_registry(text)
    try:
        value = tomllib.loads(registry)
    except tomllib.TOMLDecodeError as error:
        raise ResearchError(f"pin registry does not parse: {error}") from error
    if value.get("schema") != PIN_SCHEMA:
        raise ResearchError(
            f"pin registry schema must be {PIN_SCHEMA}, found {value.get('schema')!r}"
        )
    pins = value.get("pin", [])
    if not isinstance(pins, list) or not pins:
        raise ResearchError("pin registry has no [[pin]] rows")
    seen: set[str] = set()
    for index, row in enumerate(pins):
        if not isinstance(row, dict):
            raise ResearchError(f"pin registry row {index} is not a table")
        unknown = set(row) - PIN_KEYS
        if unknown:
            raise ResearchError(
                f"pin registry row {index} has unknown keys: {sorted(unknown)}"
            )
        identifier = row.get("id")
        if not isinstance(identifier, str) or not identifier:
            raise ResearchError(f"pin registry row {index} id must be non-empty")
        if identifier.split() != [identifier]:
            raise ResearchError(
                f"pin id must be whitespace-free, found {identifier!r}"
            )
        if identifier in seen:
            raise ResearchError(f"duplicate pin id: {identifier}")
        seen.add(identifier)
        for key in PIN_REQUIRED_KEYS:
            if key not in row:
                raise ResearchError(f"pin {identifier} is missing {key}")
        version = row.get("version")
        if not isinstance(version, str) or not version:
            raise ResearchError(f"pin {identifier} version must be non-empty")
        kind = row.get("kind")
        if kind not in KINDS:
            raise ResearchError(
                f"pin {identifier} kind must be one of {list(KINDS)},"
                f" found {kind!r}"
            )
        role = row.get("role")
        if role not in ROLES:
            raise ResearchError(
                f"pin {identifier} role must be one of {list(ROLES)},"
                f" found {role!r}"
            )
        if role not in KIND_ROLES[kind]:
            raise ResearchError(
                f"pin {identifier} role {role!r} is invalid for kind {kind!r}"
            )
        check_url(row.get("url"), identifier)
        check_retrieved(row.get("retrieved"), identifier, today)
        revision = row.get("revision", "")
        if not isinstance(revision, str):
            raise ResearchError(f"pin {identifier} revision must be a string")
        license_value = row.get("license", "")
        if kind == "model" and (not isinstance(license_value, str) or not license_value):
            raise ResearchError(
                f"pin {identifier} is a model pin and must carry a license"
            )
        if not isinstance(license_value, str):
            raise ResearchError(f"pin {identifier} license must be a string")
        if (
            kind == "model"
            and role == "primary"
            and license_value.lower().startswith("unverified")
        ):
            raise ResearchError(
                f"pin {identifier} is a PRIMARY model pin but its license is"
                f" explicitly unverified — a primary must be verified"
            )
        note = row.get("note", "")
        if not isinstance(note, str):
            raise ResearchError(f"pin {identifier} note must be a string")
        if role == "deferred" and not note:
            raise ResearchError(
                f"pin {identifier} is deferred and must carry a note explaining why"
            )
        categories = row.get("categories", [])
        if not isinstance(categories, list) or not all(
            isinstance(item, str) for item in categories
        ):
            raise ResearchError(f"pin {identifier} categories must be a string array")
        unknown_categories = set(categories) - set(CATEGORIES)
        if unknown_categories:
            raise ResearchError(
                f"pin {identifier} has unknown categories: {sorted(unknown_categories)}"
            )
        if kind == "model" and not categories:
            raise ResearchError(
                f"pin {identifier} is a model pin and must declare its workflow"
                f" categories (one of {list(CATEGORIES)})"
            )
        if kind == "tool" and categories:
            raise ResearchError(
                f"pin {identifier} is a tool pin and must not carry categories"
            )
    return pins


def check_coverage(pins: list[dict]) -> None:
    for required_tool in ("comfyui", "comfy-cli"):
        if not any(
            pin.get("id") == required_tool
            and pin.get("kind") == "tool"
            and pin.get("role") == "primary"
            for pin in pins
        ):
            raise ResearchError(
                f"pin registry lacks the required primary {required_tool} tool pin"
            )
    models = [pin for pin in pins if pin.get("kind") == "model"]
    if not models:
        raise ResearchError("pin registry has no model pins")
    for category in CATEGORIES:
        primaries = [
            pin
            for pin in models
            if category in pin.get("categories", []) and pin.get("role") == "primary"
        ]
        if not primaries:
            raise ResearchError(
                f"workflow category {category!r} has no primary model pin"
            )
    fallbacks = [
        pin
        for pin in models
        if "background-outpaint" in pin.get("categories", [])
        and pin.get("role") == "fallback"
    ]
    if not fallbacks:
        raise ResearchError(
            "workflow category 'background-outpaint' has no fallback model pin"
        )


def check_manifest(root: Path) -> None:
    path = root / MANIFEST_RELATIVE
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ResearchError(f"required-gates manifest is missing: {path}") from error
    try:
        manifest = tomllib.loads(raw.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise ResearchError(f"required-gates manifest does not parse: {error}") from error
    phases = {
        phase.get("id"): phase
        for phase in manifest.get("phase", [])
        if isinstance(phase, dict)
    }
    p6 = phases.get("P6")
    if p6 is None:
        raise ResearchError("required-gates manifest has no P6 phase")
    required = p6.get("required_gates", [])
    if not isinstance(required, list) or GATE_ID not in required:
        raise ResearchError(f"P6 required_gates does not include {GATE_ID}")
    gates = {
        gate.get("id"): gate
        for gate in manifest.get("gate", [])
        if isinstance(gate, dict)
    }
    gate = gates.get(GATE_ID)
    if gate is None:
        raise ResearchError(
            f"P6 required_gates names {GATE_ID} but no [[gate]] with that id"
            " is defined"
        )
    commands = gate.get("commands", [])
    if not isinstance(commands, list):
        raise ResearchError(f"gate {GATE_ID} commands must be an array")
    runs_checker = any(
        isinstance(command, list) and CHECKER_RELATIVE in command
        for command in commands
    )
    if not runs_checker:
        raise ResearchError(
            f"gate {GATE_ID} commands do not run {CHECKER_RELATIVE}"
        )
    tracked = gate.get("tracked_paths", [])
    if not isinstance(tracked, list) or DOC_RELATIVE not in tracked:
        raise ResearchError(
            f"gate {GATE_ID} tracked_paths do not include {DOC_RELATIVE}"
        )


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=default_root)
    arguments = parser.parse_args()
    root = arguments.root.resolve(strict=True)
    today = datetime.datetime.now(datetime.timezone.utc).date()
    try:
        text = load_doc(root)
        check_sections_and_sentences(text)
        pins = load_pins(text, today)
        check_coverage(pins)
        check_manifest(root)
    except ResearchError as error:
        print(f"p6-hd-asset-research: FAIL: {error}", file=sys.stderr)
        return 1
    by_kind: dict[str, int] = {}
    by_role: dict[str, int] = {}
    covered: dict[str, list[str]] = {category: [] for category in CATEGORIES}
    for pin in pins:
        by_kind[pin["kind"]] = by_kind.get(pin["kind"], 0) + 1
        by_role[pin["role"]] = by_role.get(pin["role"], 0) + 1
        if pin["kind"] == "model":
            for category in pin.get("categories", []):
                covered[category].append(
                    f"{pin['id']}:{pin['role']}"
                )
    print("p6-hd-asset-research: OK")
    print(
        "  pins: "
        + ", ".join(f"{kind} {count}" for kind, count in sorted(by_kind.items()))
        + " ("
        + ", ".join(f"{role} {count}" for role, count in sorted(by_role.items()))
        + ")"
    )
    for category in CATEGORIES:
        print(f"  category {category}: {' '.join(sorted(covered[category]))}")
    print("  rules: sections + boundary sentences + manifest wiring verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

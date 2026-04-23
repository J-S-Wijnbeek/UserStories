"""
Dwarf Fortress mod validator.

Parses mod directories containing info.txt and objects/*.txt files, then
checks them for structural correctness and cross-reference consistency.
"""

from __future__ import annotations

import re
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterator


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------

@dataclass
class Issue:
    severity: str  # "ERROR" or "WARNING"
    path: str
    line: int | None
    message: str

    def __str__(self) -> str:
        loc = f"{self.path}:{self.line}" if self.line is not None else self.path
        return f"[{self.severity}] {loc}: {self.message}"


@dataclass
class ModInfo:
    path: Path
    fields: dict[str, str] = field(default_factory=dict)


@dataclass
class RawObject:
    """A single object definition extracted from a DF raw file."""
    obj_type: str       # e.g. CREATURE, ENTITY, INORGANIC, REACTION, BUILDING
    identifier: str     # e.g. HIVE_DRONE
    tokens: list[tuple[str, list[str]]]  # [(token_name, [args, ...]), ...]
    source_file: Path
    start_line: int


@dataclass
class ParsedFile:
    path: Path
    object_type: str | None       # value of [OBJECT:...] declaration
    objects: list[RawObject]


# ---------------------------------------------------------------------------
# Info.txt validation
# ---------------------------------------------------------------------------

INFO_REQUIRED_FIELDS = [
    "ID",
    "NUMERIC_VERSION",
    "DISPLAYED_VERSION",
    "NAME",
    "AUTHOR",
    "DESCRIPTION",
]

_TOKEN_RE = re.compile(r"^\[([A-Z0-9_]+)(?::([^\]]*))?\]")


def parse_info_txt(path: Path) -> tuple[dict[str, str], list[Issue]]:
    """Parse an info.txt file and return (fields, issues)."""
    fields: dict[str, str] = {}
    issues: list[Issue] = []

    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        issues.append(Issue("ERROR", str(path), None, f"Cannot read file: {exc}"))
        return fields, issues

    for lineno, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.strip()
        if not line:
            continue
        for m in re.finditer(r"\[([A-Z][A-Z0-9_]*)(?::([^\]]*))?\]", line):
            key = m.group(1)
            value = m.group(2) or ""
            fields[key] = value
        # non-tag lines inside info.txt are description continuations — ok

    for req in INFO_REQUIRED_FIELDS:
        if req not in fields:
            issues.append(Issue("ERROR", str(path), None,
                                f"info.txt missing required field [{req}:]"))

    return fields, issues


# ---------------------------------------------------------------------------
# Raw object file parsing
# ---------------------------------------------------------------------------

KNOWN_OBJECT_TYPES = {
    "CREATURE", "ENTITY", "INORGANIC", "REACTION", "BUILDING",
    "ITEM", "BODY", "BODY_DETAIL_PLAN", "TISSUE", "MATERIAL_TEMPLATE",
    "CREATURE_VARIATION", "INTERACTION", "LANGUAGE", "TRANSLATION",
    "MUSIC", "SOUND", "DESCRIPTOR_COLOR", "DESCRIPTOR_SHAPE",
    "DESCRIPTOR_PATTERN", "PLANT", "WORD", "SYMBOL", "VERB",
}

# First object-level token per type
_OBJECT_HEADER = {
    "CREATURE":   "CREATURE",
    "ENTITY":     "ENTITY",
    "INORGANIC":  "INORGANIC",
    "REACTION":   "REACTION",
    "BUILDING":   "BUILDING_WORKSHOP",
}


def _iter_tokens(text: str) -> Iterator[tuple[int, str, list[str]]]:
    """Yield (lineno, token_name, args_list) for every [TOKEN:…] in text."""
    for lineno, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.strip()
        for m in re.finditer(r"\[([A-Z][A-Z0-9_]*)(?::([^\]]*))?\]", line):
            name = m.group(1)
            raw_args = m.group(2)
            args = [a.strip() for a in raw_args.split(":")] if raw_args else []
            yield lineno, name, args


def parse_raw_file(path: Path) -> tuple[ParsedFile, list[Issue]]:
    """Parse a DF raw object file and return (ParsedFile, issues)."""
    issues: list[Issue] = []
    objects: list[RawObject] = []
    object_type: str | None = None

    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:
        issues.append(Issue("ERROR", str(path), None, f"Cannot read file: {exc}"))
        return ParsedFile(path, None, []), issues

    current_obj: RawObject | None = None

    for lineno, token, args in _iter_tokens(text):
        # Detect [OBJECT:TYPE]
        if token == "OBJECT":
            if not args:
                issues.append(Issue("ERROR", str(path), lineno,
                                    "[OBJECT] tag has no type argument"))
                continue
            declared = args[0]
            if declared not in KNOWN_OBJECT_TYPES:
                issues.append(Issue("WARNING", str(path), lineno,
                                    f"Unknown [OBJECT:{declared}]; "
                                    "expected one of: " + ", ".join(sorted(KNOWN_OBJECT_TYPES))))
            if object_type is not None and declared != object_type:
                issues.append(Issue("ERROR", str(path), lineno,
                                    f"Multiple [OBJECT:] declarations in one file "
                                    f"({object_type!r} then {declared!r})"))
            object_type = declared
            continue

        if object_type is None:
            # tokens before any [OBJECT:] declaration
            continue

        expected_header = _OBJECT_HEADER.get(object_type)

        # Start of a new top-level object
        if expected_header and token == expected_header:
            if not args:
                issues.append(Issue("ERROR", str(path), lineno,
                                    f"[{token}] tag missing identifier"))
                continue
            if current_obj is not None:
                objects.append(current_obj)
            current_obj = RawObject(
                obj_type=object_type,
                identifier=args[0],
                tokens=[],
                source_file=path,
                start_line=lineno,
            )
            continue

        if current_obj is not None:
            current_obj.tokens.append((token, args))

    if current_obj is not None:
        objects.append(current_obj)

    if object_type is None:
        issues.append(Issue("WARNING", str(path), None,
                            "No [OBJECT:TYPE] declaration found in file"))

    return ParsedFile(path, object_type, objects), issues


# ---------------------------------------------------------------------------
# Per-type object validation
# ---------------------------------------------------------------------------

def _has_token(obj: RawObject, token_name: str) -> bool:
    return any(t == token_name for t, _ in obj.tokens)


def _token_args(obj: RawObject, token_name: str) -> list[str] | None:
    for t, args in obj.tokens:
        if t == token_name:
            return args
    return None


def _validate_creature(obj: RawObject) -> list[Issue]:
    issues: list[Issue] = []
    p = str(obj.source_file)

    for required in ("NAME", "DESCRIPTION", "CREATURE_TILE", "BODY", "BODY_SIZE", "MAXAGE"):
        if not _has_token(obj, required):
            issues.append(Issue("ERROR", p, obj.start_line,
                                f"CREATURE:{obj.identifier} missing required [{required}:]"))

    maxage = _token_args(obj, "MAXAGE")
    if maxage and len(maxage) == 2:
        try:
            lo, hi = int(maxage[0]), int(maxage[1])
            if lo > hi:
                issues.append(Issue("ERROR", p, obj.start_line,
                                    f"CREATURE:{obj.identifier} [MAXAGE:{lo}:{hi}] "
                                    "minimum age exceeds maximum age"))
        except ValueError:
            pass

    return issues


def _validate_entity(obj: RawObject) -> list[Issue]:
    issues: list[Issue] = []
    p = str(obj.source_file)

    if not _has_token(obj, "CREATURE"):
        issues.append(Issue("ERROR", p, obj.start_line,
                            f"ENTITY:{obj.identifier} missing required [CREATURE:] token"))

    if _has_token(obj, "SITE_CONTROLLABLE") and not _has_token(obj, "TRANSLATION"):
        issues.append(Issue("WARNING", p, obj.start_line,
                            f"ENTITY:{obj.identifier} is [SITE_CONTROLLABLE] but has no "
                            "[TRANSLATION:] token (dwarven language will be used by default)"))

    return issues


def _validate_inorganic(obj: RawObject) -> list[Issue]:
    issues: list[Issue] = []
    p = str(obj.source_file)

    if not _has_token(obj, "STATE_NAME_ADJ"):
        issues.append(Issue("ERROR", p, obj.start_line,
                            f"INORGANIC:{obj.identifier} missing required [STATE_NAME_ADJ:]"))

    # Both IS_STONE and IS_METAL together is unusual but not invalid; warn.
    if _has_token(obj, "IS_STONE") and _has_token(obj, "IS_METAL"):
        issues.append(Issue("WARNING", p, obj.start_line,
                            f"INORGANIC:{obj.identifier} has both [IS_STONE] and [IS_METAL]; "
                            "this is unusual and may cause stockpile issues"))

    return issues


def _validate_reaction(obj: RawObject) -> list[Issue]:
    issues: list[Issue] = []
    p = str(obj.source_file)

    for required in ("NAME", "BUILDING"):
        if not _has_token(obj, required):
            issues.append(Issue("ERROR", p, obj.start_line,
                                f"REACTION:{obj.identifier} missing required [{required}:]"))

    if not _has_token(obj, "REAGENT"):
        issues.append(Issue("ERROR", p, obj.start_line,
                            f"REACTION:{obj.identifier} has no [REAGENT:] — "
                            "reactions need at least one reagent"))

    if not _has_token(obj, "PRODUCT"):
        issues.append(Issue("ERROR", p, obj.start_line,
                            f"REACTION:{obj.identifier} has no [PRODUCT:] — "
                            "reactions need at least one product"))

    # Validate PRODUCT probability (first arg should be 0-100)
    for token, args in obj.tokens:
        if token == "PRODUCT" and args:
            try:
                prob = int(args[0])
                if not 0 <= prob <= 100:
                    issues.append(Issue("ERROR", p, obj.start_line,
                                        f"REACTION:{obj.identifier} [PRODUCT] probability "
                                        f"{prob} is outside 0–100 range"))
            except ValueError:
                issues.append(Issue("ERROR", p, obj.start_line,
                                    f"REACTION:{obj.identifier} [PRODUCT] first argument "
                                    f"{args[0]!r} is not an integer probability"))

    return issues


def _validate_building(obj: RawObject) -> list[Issue]:
    issues: list[Issue] = []
    p = str(obj.source_file)

    for required in ("NAME", "DIM", "WORK_LOCATION"):
        if not _has_token(obj, required):
            issues.append(Issue("ERROR", p, obj.start_line,
                                f"BUILDING_WORKSHOP:{obj.identifier} missing required "
                                f"[{required}:]"))

    dim = _token_args(obj, "DIM")
    if dim and len(dim) == 2:
        try:
            w, h = int(dim[0]), int(dim[1])
            if w <= 0 or h <= 0:
                issues.append(Issue("ERROR", p, obj.start_line,
                                    f"BUILDING_WORKSHOP:{obj.identifier} [DIM:{w}:{h}] "
                                    "dimensions must be positive"))
        except ValueError:
            pass

    return issues


_VALIDATORS = {
    "CREATURE": _validate_creature,
    "ENTITY": _validate_entity,
    "INORGANIC": _validate_inorganic,
    "REACTION": _validate_reaction,
    "BUILDING": _validate_building,
}


def validate_object(obj: RawObject) -> list[Issue]:
    validator = _VALIDATORS.get(obj.obj_type)
    if validator:
        return validator(obj)
    return []


# ---------------------------------------------------------------------------
# Cross-reference validation
# ---------------------------------------------------------------------------

def cross_validate(all_files: list[ParsedFile]) -> list[Issue]:
    """Check inter-file references for consistency."""
    issues: list[Issue] = []

    known_creatures: set[str] = set()
    known_inorganics: set[str] = set()
    known_buildings: set[str] = set()

    for pf in all_files:
        for obj in pf.objects:
            if obj.obj_type == "CREATURE":
                known_creatures.add(obj.identifier)
            elif obj.obj_type == "INORGANIC":
                known_inorganics.add(obj.identifier)
            elif obj.obj_type == "BUILDING":
                known_buildings.add(obj.identifier)

    # Validate entity → creature references
    for pf in all_files:
        for obj in pf.objects:
            if obj.obj_type != "ENTITY":
                continue
            for token, args in obj.tokens:
                if token == "CREATURE" and args:
                    cid = args[0]
                    if cid not in known_creatures:
                        issues.append(Issue(
                            "ERROR", str(obj.source_file), obj.start_line,
                            f"ENTITY:{obj.identifier} references unknown "
                            f"[CREATURE:{cid}] — not defined in any mod file"))

    # Validate reaction → building and inorganic references
    for pf in all_files:
        for obj in pf.objects:
            if obj.obj_type != "REACTION":
                continue
            for token, args in obj.tokens:
                if token == "BUILDING" and args:
                    bid = args[0]
                    if bid not in known_buildings:
                        issues.append(Issue(
                            "ERROR", str(obj.source_file), obj.start_line,
                            f"REACTION:{obj.identifier} references unknown "
                            f"[BUILDING:{bid}] — not defined in any mod file"))

                # Check INORGANIC material references in REAGENT / PRODUCT
                if token in ("REAGENT", "PRODUCT"):
                    # REAGENT: name:qty:item_type:item_subtype:mat_type:mat_id
                    # PRODUCT: prob:qty:item_type:item_subtype:mat_type:mat_id
                    if len(args) >= 6:
                        mat_type = args[4]
                        mat_id = args[5]
                        if mat_type == "INORGANIC" and mat_id not in ("NONE", "ALL"):
                            if mat_id not in known_inorganics:
                                issues.append(Issue(
                                    "ERROR", str(obj.source_file), obj.start_line,
                                    f"REACTION:{obj.identifier} [{token}] references "
                                    f"unknown [INORGANIC:{mat_id}]"))

    # Validate building → inorganic build item references
    for pf in all_files:
        for obj in pf.objects:
            if obj.obj_type != "BUILDING":
                continue
            for token, args in obj.tokens:
                if token == "BUILD_ITEM" and len(args) >= 5:
                    mat_type = args[3]
                    mat_id = args[4]
                    if mat_type == "INORGANIC" and mat_id not in ("NONE", "ALL"):
                        if mat_id not in known_inorganics:
                            issues.append(Issue(
                                "ERROR", str(obj.source_file), obj.start_line,
                                f"BUILDING_WORKSHOP:{obj.identifier} [BUILD_ITEM] references "
                                f"unknown [INORGANIC:{mat_id}]"))

    return issues


# ---------------------------------------------------------------------------
# Mod directory discovery and top-level validation
# ---------------------------------------------------------------------------

def find_mod_dirs(root: Path) -> list[Path]:
    """Return all directories under root that look like a DF mod."""
    mods: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dp = Path(dirpath)
        if "info.txt" in filenames and (dp / "objects").is_dir():
            mods.append(dp)
            dirnames.clear()  # don't recurse into mod subdirs
    return mods


def validate_mod(mod_dir: Path) -> list[Issue]:
    """Validate a single mod directory and return all issues found."""
    issues: list[Issue] = []

    # --- info.txt ---
    info_path = mod_dir / "info.txt"
    _, info_issues = parse_info_txt(info_path)
    issues.extend(info_issues)

    # --- object files ---
    objects_dir = mod_dir / "objects"
    all_parsed: list[ParsedFile] = []

    for txt_file in sorted(objects_dir.glob("*.txt")):
        parsed, file_issues = parse_raw_file(txt_file)
        issues.extend(file_issues)
        all_parsed.append(parsed)

        for obj in parsed.objects:
            issues.extend(validate_object(obj))

    # --- cross-reference ---
    issues.extend(cross_validate(all_parsed))

    return issues


def validate_all(root: Path) -> dict[Path, list[Issue]]:
    """Discover all mods under root and validate each one."""
    results: dict[Path, list[Issue]] = {}
    for mod_dir in find_mod_dirs(root):
        results[mod_dir] = validate_mod(mod_dir)
    return results

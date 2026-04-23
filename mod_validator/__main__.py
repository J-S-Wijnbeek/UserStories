"""CLI entry point: python -m mod_validator [path]"""

import sys
from pathlib import Path

from .validator import validate_all, validate_mod, find_mod_dirs


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")

    if not root.exists():
        print(f"Error: path not found: {root}", file=sys.stderr)
        return 1

    # If the given path is itself a mod dir, validate just it.
    if (root / "info.txt").exists() and (root / "objects").is_dir():
        mod_dirs = [root]
    else:
        mod_dirs = find_mod_dirs(root)

    if not mod_dirs:
        print(f"No mod directories found under: {root}", file=sys.stderr)
        return 1

    total_errors = 0
    total_warnings = 0

    for mod_dir in mod_dirs:
        issues = validate_mod(mod_dir)
        errors = [i for i in issues if i.severity == "ERROR"]
        warnings = [i for i in issues if i.severity == "WARNING"]
        total_errors += len(errors)
        total_warnings += len(warnings)

        print(f"\n=== {mod_dir} ===")
        if not issues:
            print("  OK — no issues found.")
        else:
            for issue in sorted(issues, key=lambda i: (i.path, i.line or 0)):
                print(f"  {issue}")

    print(
        f"\nSummary: {len(mod_dirs)} mod(s) checked, "
        f"{total_errors} error(s), {total_warnings} warning(s)."
    )
    return 1 if total_errors else 0


if __name__ == "__main__":
    sys.exit(main())

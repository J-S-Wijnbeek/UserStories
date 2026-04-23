# UserStories

A collection of Dwarf Fortress mod files and tools.

## Mods

| Directory | Description |
|---|---|
| `duergar_module (10)/` | PRR Module — Duergar race (standalone) |
| `mods/hivemind_mod/` | Biomechanical hivemind fortress conversion |

## Mod Validator

`mod_validator/` is a Python tool that reads the DF mod files in this repo
and checks them for structural correctness and cross-reference consistency.

**What it checks:**

- `info.txt` required fields (`ID`, `NUMERIC_VERSION`, `DISPLAYED_VERSION`, `NAME`, `AUTHOR`, `DESCRIPTION`)
- Raw object files — `[OBJECT:TYPE]` declaration and per-type required tokens
  - `CREATURE`: `NAME`, `DESCRIPTION`, `CREATURE_TILE`, `BODY`, `BODY_SIZE`, `MAXAGE` (and sane age range)
  - `ENTITY`: must reference at least one `[CREATURE:]`; warns if `[SITE_CONTROLLABLE]` without `[TRANSLATION:]`
  - `INORGANIC`: `STATE_NAME_ADJ`; warns if `[IS_STONE]` + `[IS_METAL]` together
  - `REACTION`: `NAME`, `BUILDING`, at least one `REAGENT` and `PRODUCT`, valid probability (0–100)
  - `BUILDING_WORKSHOP`: `NAME`, `DIM`, `WORK_LOCATION` with positive dimensions
- Cross-references — entities → creatures, reactions → buildings, reactions/buildings → inorganics

**Usage:**

```sh
# Validate all mods under the repo root
python -m mod_validator .

# Validate a single mod directory
python -m mod_validator "duergar_module (10)"
```

Exit code is `0` if only warnings (or clean), `1` if any errors.

**Tests:**

```sh
python -m unittest discover -s mod_validator/tests -v
```

## fortress_sim

A Dwarf Fortress-inspired game engine simulation written in Rust.

```sh
cargo build --manifest-path fortress_sim/Cargo.toml
cargo test  --manifest-path fortress_sim/Cargo.toml
```

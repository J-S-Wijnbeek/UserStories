"""Unit tests for the DF mod validator."""

import textwrap
import unittest
from pathlib import Path
from unittest.mock import mock_open, patch

from mod_validator.validator import (
    Issue,
    RawObject,
    ParsedFile,
    parse_info_txt,
    parse_raw_file,
    validate_object,
    cross_validate,
    validate_mod,
    find_mod_dirs,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_raw_obj(obj_type: str, identifier: str, tokens: list[tuple[str, list[str]]],
                  path: Path | None = None, start_line: int = 1) -> RawObject:
    return RawObject(
        obj_type=obj_type,
        identifier=identifier,
        tokens=tokens,
        source_file=path or Path("test.txt"),
        start_line=start_line,
    )


def _errors(issues: list[Issue]) -> list[Issue]:
    return [i for i in issues if i.severity == "ERROR"]


def _warnings(issues: list[Issue]) -> list[Issue]:
    return [i for i in issues if i.severity == "WARNING"]


# ---------------------------------------------------------------------------
# info.txt parsing
# ---------------------------------------------------------------------------

class TestParseInfoTxt(unittest.TestCase):

    def _parse(self, content: str) -> tuple[dict, list[Issue]]:
        import tempfile, os
        with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
            f.write(content)
            tmp = Path(f.name)
        try:
            return parse_info_txt(tmp)
        finally:
            os.unlink(tmp)

    def test_valid_info(self):
        content = textwrap.dedent("""\
            [ID:my_mod]
            [NUMERIC_VERSION:1]
            [DISPLAYED_VERSION:1.0]
            [NAME:My Mod]
            [AUTHOR:Tester]
            [DESCRIPTION:A test mod.]
        """)
        fields, issues = self._parse(content)
        self.assertEqual(fields["ID"], "my_mod")
        self.assertEqual(fields["NAME"], "My Mod")
        self.assertEqual(_errors(issues), [])

    def test_missing_required_fields(self):
        content = "[ID:my_mod]\n"
        _, issues = self._parse(content)
        missing = {i.message for i in _errors(issues)}
        self.assertIn("info.txt missing required field [NUMERIC_VERSION:]", missing)
        self.assertIn("info.txt missing required field [NAME:]", missing)
        self.assertIn("info.txt missing required field [AUTHOR:]", missing)
        self.assertIn("info.txt missing required field [DESCRIPTION:]", missing)

    def test_all_required_fields_present(self):
        content = textwrap.dedent("""\
            [ID:x][NUMERIC_VERSION:1][DISPLAYED_VERSION:1.0]
            [NAME:X][AUTHOR:Y][DESCRIPTION:Z]
        """)
        _, issues = self._parse(content)
        self.assertEqual(_errors(issues), [])

    def test_non_tag_lines_ignored(self):
        content = textwrap.dedent("""\
            [ID:ok]
            [NUMERIC_VERSION:1]
            [DISPLAYED_VERSION:1.0]
            [NAME:OK]
            [AUTHOR:Author]
            [DESCRIPTION:Desc]
            This is free-text that should not cause errors.
        """)
        _, issues = self._parse(content)
        self.assertEqual(_errors(issues), [])


# ---------------------------------------------------------------------------
# Raw file parsing
# ---------------------------------------------------------------------------

class TestParseRawFile(unittest.TestCase):

    def _parse(self, content: str) -> tuple[ParsedFile, list[Issue]]:
        import tempfile, os
        with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
            f.write(content)
            tmp = Path(f.name)
        try:
            return parse_raw_file(tmp)
        finally:
            os.unlink(tmp)

    def test_basic_creature_file(self):
        content = textwrap.dedent("""\
            creature_test

            [OBJECT:CREATURE]

            [CREATURE:MY_CRITTER]
            [NAME:critter:critters:critter]
            [DESCRIPTION:A test creature.]
            [CREATURE_TILE:'x']
            [BODY:HUMANOID]
            [BODY_SIZE:0:0:50000]
            [MAXAGE:20:30]
        """)
        pf, issues = self._parse(content)
        self.assertEqual(pf.object_type, "CREATURE")
        self.assertEqual(len(pf.objects), 1)
        self.assertEqual(pf.objects[0].identifier, "MY_CRITTER")
        self.assertEqual(_errors(issues), [])

    def test_multiple_creatures(self):
        content = textwrap.dedent("""\
            [OBJECT:CREATURE]
            [CREATURE:A]
            [NAME:a:as:a]
            [CREATURE:B]
            [NAME:b:bs:b]
        """)
        pf, _ = self._parse(content)
        self.assertEqual(len(pf.objects), 2)
        self.assertEqual(pf.objects[0].identifier, "A")
        self.assertEqual(pf.objects[1].identifier, "B")

    def test_missing_object_declaration(self):
        content = "[CREATURE:ORPHAN]\n[NAME:x:x:x]\n"
        pf, issues = self._parse(content)
        self.assertTrue(any(i.severity == "WARNING" and "No [OBJECT:TYPE]" in i.message
                            for i in issues))

    def test_unknown_object_type_warns(self):
        content = "[OBJECT:BANANAS]\n"
        _, issues = self._parse(content)
        self.assertTrue(any(i.severity == "WARNING" and "BANANAS" in i.message
                            for i in issues))

    def test_object_tag_without_type_errors(self):
        content = "[OBJECT:]\n"
        _, issues = self._parse(content)
        # empty arg → treated as empty string, which is not in KNOWN_OBJECT_TYPES
        # (the code accepts "" as args[0] which becomes ""), should warn
        self.assertTrue(any("OBJECT" in i.message for i in issues))


# ---------------------------------------------------------------------------
# Per-type validation
# ---------------------------------------------------------------------------

class TestValidateCreature(unittest.TestCase):

    def _good_tokens(self) -> list[tuple[str, list[str]]]:
        return [
            ("NAME", ["critter", "critters", "critter"]),
            ("DESCRIPTION", ["A test."]),
            ("CREATURE_TILE", ["'x'"]),
            ("BODY", ["HUMANOID"]),
            ("BODY_SIZE", ["0", "0", "50000"]),
            ("MAXAGE", ["20", "30"]),
        ]

    def test_valid_creature(self):
        obj = _make_raw_obj("CREATURE", "TEST", self._good_tokens())
        self.assertEqual(_errors(validate_object(obj)), [])

    def test_missing_name(self):
        tokens = [t for t in self._good_tokens() if t[0] != "NAME"]
        obj = _make_raw_obj("CREATURE", "TEST", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("NAME" in m for m in msgs))

    def test_missing_body(self):
        tokens = [t for t in self._good_tokens() if t[0] != "BODY"]
        obj = _make_raw_obj("CREATURE", "TEST", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("BODY" in m for m in msgs))

    def test_inverted_maxage_errors(self):
        tokens = [t for t in self._good_tokens() if t[0] != "MAXAGE"]
        tokens.append(("MAXAGE", ["50", "20"]))  # min > max
        obj = _make_raw_obj("CREATURE", "TEST", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("MAXAGE" in m and "minimum age exceeds maximum age" in m
                            for m in msgs))


class TestValidateEntity(unittest.TestCase):

    def test_valid_entity(self):
        obj = _make_raw_obj("ENTITY", "MY_CIV", [
            ("CREATURE", ["MY_POP"]),
            ("SITE_CONTROLLABLE", []),
            ("TRANSLATION", ["DWARF"]),
        ])
        self.assertEqual(_errors(validate_object(obj)), [])

    def test_missing_creature(self):
        obj = _make_raw_obj("ENTITY", "EMPTY_CIV", [
            ("SITE_CONTROLLABLE", []),
            ("TRANSLATION", ["DWARF"]),
        ])
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("CREATURE" in m for m in msgs))

    def test_site_controllable_without_translation_warns(self):
        obj = _make_raw_obj("ENTITY", "NO_LANG_CIV", [
            ("CREATURE", ["POP"]),
            ("SITE_CONTROLLABLE", []),
        ])
        msgs = [i.message for i in _warnings(validate_object(obj))]
        self.assertTrue(any("TRANSLATION" in m for m in msgs))


class TestValidateInorganic(unittest.TestCase):

    def test_valid_inorganic(self):
        obj = _make_raw_obj("INORGANIC", "BIOMASSA", [
            ("USE_MATERIAL_TEMPLATE", ["STONE_TEMPLATE"]),
            ("STATE_NAME_ADJ", ["ALL_SOLID", "biomassa"]),
            ("IS_STONE", []),
        ])
        self.assertEqual(_errors(validate_object(obj)), [])

    def test_missing_state_name_adj(self):
        obj = _make_raw_obj("INORGANIC", "WEIRD", [("IS_STONE", [])])
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("STATE_NAME_ADJ" in m for m in msgs))

    def test_is_stone_and_is_metal_warns(self):
        obj = _make_raw_obj("INORGANIC", "WEIRD", [
            ("STATE_NAME_ADJ", ["ALL_SOLID", "weird"]),
            ("IS_STONE", []),
            ("IS_METAL", []),
        ])
        msgs = [i.message for i in _warnings(validate_object(obj))]
        self.assertTrue(any("IS_STONE" in m and "IS_METAL" in m for m in msgs))


class TestValidateReaction(unittest.TestCase):

    def _good_tokens(self) -> list[tuple[str, list[str]]]:
        return [
            ("NAME", ["make stuff"]),
            ("BUILDING", ["MY_WORKSHOP", "CUSTOM_A"]),
            ("REAGENT", ["ore", "1", "BOULDER", "NONE", "INORGANIC", "NONE"]),
            ("PRODUCT", ["100", "1", "BAR", "NONE", "INORGANIC", "MYMAT"]),
            ("SKILL", ["SMELT"]),
        ]

    def test_valid_reaction(self):
        obj = _make_raw_obj("REACTION", "MAKE_STUFF", self._good_tokens())
        self.assertEqual(_errors(validate_object(obj)), [])

    def test_missing_reagent(self):
        tokens = [t for t in self._good_tokens() if t[0] != "REAGENT"]
        obj = _make_raw_obj("REACTION", "BAD", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("REAGENT" in m for m in msgs))

    def test_missing_product(self):
        tokens = [t for t in self._good_tokens() if t[0] != "PRODUCT"]
        obj = _make_raw_obj("REACTION", "BAD", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("PRODUCT" in m for m in msgs))

    def test_product_probability_out_of_range(self):
        tokens = [t for t in self._good_tokens() if t[0] != "PRODUCT"]
        tokens.append(("PRODUCT", ["150", "1", "BAR", "NONE", "INORGANIC", "X"]))
        obj = _make_raw_obj("REACTION", "BAD", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("probability" in m and "150" in m for m in msgs))


class TestValidateBuilding(unittest.TestCase):

    def _good_tokens(self) -> list[tuple[str, list[str]]]:
        return [
            ("NAME", ["My Shop"]),
            ("DIM", ["3", "3"]),
            ("WORK_LOCATION", ["1", "1"]),
        ]

    def test_valid_building(self):
        obj = _make_raw_obj("BUILDING", "MY_SHOP", self._good_tokens())
        self.assertEqual(_errors(validate_object(obj)), [])

    def test_missing_dim(self):
        tokens = [t for t in self._good_tokens() if t[0] != "DIM"]
        obj = _make_raw_obj("BUILDING", "MY_SHOP", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("DIM" in m for m in msgs))

    def test_zero_dim_errors(self):
        tokens = [t for t in self._good_tokens() if t[0] != "DIM"]
        tokens.append(("DIM", ["0", "3"]))
        obj = _make_raw_obj("BUILDING", "MY_SHOP", tokens)
        msgs = [i.message for i in _errors(validate_object(obj))]
        self.assertTrue(any("dimensions must be positive" in m for m in msgs))


# ---------------------------------------------------------------------------
# Cross-reference validation
# ---------------------------------------------------------------------------

class TestCrossValidate(unittest.TestCase):

    def _make_pf(self, objects: list[RawObject]) -> ParsedFile:
        return ParsedFile(path=Path("dummy.txt"), object_type=None, objects=objects)

    def test_entity_references_known_creature(self):
        creature = _make_raw_obj("CREATURE", "MY_POP", [])
        entity = _make_raw_obj("ENTITY", "MY_CIV", [("CREATURE", ["MY_POP"])])
        issues = cross_validate([self._make_pf([creature]), self._make_pf([entity])])
        self.assertEqual(_errors(issues), [])

    def test_entity_references_unknown_creature(self):
        entity = _make_raw_obj("ENTITY", "MY_CIV", [("CREATURE", ["GHOST_POP"])])
        issues = cross_validate([self._make_pf([entity])])
        msgs = [i.message for i in _errors(issues)]
        self.assertTrue(any("GHOST_POP" in m for m in msgs))

    def test_reaction_references_known_building(self):
        building = _make_raw_obj("BUILDING", "MY_SHOP", [])
        reaction = _make_raw_obj("REACTION", "MY_REACT", [
            ("NAME", ["do thing"]),
            ("BUILDING", ["MY_SHOP", "CUSTOM_A"]),
            ("REAGENT", ["ore", "1", "BOULDER", "NONE", "INORGANIC", "NONE"]),
            ("PRODUCT", ["100", "1", "BAR", "NONE", "INORGANIC", "NONE"]),
        ])
        issues = cross_validate([self._make_pf([building]), self._make_pf([reaction])])
        self.assertEqual(_errors(issues), [])

    def test_reaction_references_unknown_building(self):
        reaction = _make_raw_obj("REACTION", "MY_REACT", [
            ("BUILDING", ["GHOST_SHOP", "CUSTOM_A"]),
        ])
        issues = cross_validate([self._make_pf([reaction])])
        msgs = [i.message for i in _errors(issues)]
        self.assertTrue(any("GHOST_SHOP" in m for m in msgs))

    def test_reaction_references_unknown_inorganic(self):
        building = _make_raw_obj("BUILDING", "SHOP", [])
        reaction = _make_raw_obj("REACTION", "MY_REACT", [
            ("BUILDING", ["SHOP", "CUSTOM_A"]),
            ("PRODUCT", ["100", "1", "BAR", "NONE", "INORGANIC", "GHOST_MAT"]),
        ])
        issues = cross_validate([self._make_pf([building]), self._make_pf([reaction])])
        msgs = [i.message for i in _errors(issues)]
        self.assertTrue(any("GHOST_MAT" in m for m in msgs))

    def test_reaction_inorganic_none_skipped(self):
        building = _make_raw_obj("BUILDING", "SHOP", [])
        reaction = _make_raw_obj("REACTION", "MY_REACT", [
            ("BUILDING", ["SHOP", "CUSTOM_A"]),
            ("REAGENT", ["ore", "1", "BOULDER", "NONE", "INORGANIC", "NONE"]),
            ("PRODUCT", ["100", "1", "BAR", "NONE", "INORGANIC", "NONE"]),
        ])
        issues = cross_validate([self._make_pf([building]), self._make_pf([reaction])])
        self.assertEqual(_errors(issues), [])


# ---------------------------------------------------------------------------
# Integration test against the actual repository mods
# ---------------------------------------------------------------------------

class TestRealMods(unittest.TestCase):
    """Smoke-test the validator against the actual mod directories in the repo."""

    REPO_ROOT = Path(__file__).resolve().parents[2]

    def test_duergar_mod_no_errors(self):
        mod_dir = self.REPO_ROOT / "duergar_module (10)"
        if not mod_dir.exists():
            self.skipTest("duergar_module not found")
        issues = validate_mod(mod_dir)
        errors = _errors(issues)
        self.assertEqual(errors, [],
                         msg="\n".join(str(i) for i in errors))

    def test_hivemind_mod_no_errors(self):
        mod_dir = self.REPO_ROOT / "mods" / "hivemind_mod"
        if not mod_dir.exists():
            self.skipTest("hivemind_mod not found")
        issues = validate_mod(mod_dir)
        errors = _errors(issues)
        self.assertEqual(errors, [],
                         msg="\n".join(str(i) for i in errors))

    def test_find_mod_dirs_discovers_both(self):
        mods = find_mod_dirs(self.REPO_ROOT)
        names = {m.name for m in mods}
        self.assertIn("duergar_module (10)", names)
        self.assertIn("hivemind_mod", names)


if __name__ == "__main__":
    unittest.main()

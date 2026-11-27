"""Unit tests for the verification driver. Run: python3 -m unittest discover -s scripts"""

import json
import unittest
from pathlib import Path

import verify
from verify_checks import (evaluate, find_center, has_key, has_text, moved, no_key,
                           opacity_between, overflowing_text, page_keys, text_count, text_fits, covers_window)

ROOT = Path(__file__).resolve().parent.parent


def tree(*elements):
    return {"elements": list(elements)}


def el(key=None, text=None, opacity=1.0, rect=(0, 0, 10, 10), role="group", label=None):
    return {"key": key, "text": text, "opacity": opacity, "rect": list(rect), "role": role, "label": label}


class ChecksTest(unittest.TestCase):
    def passed(self, check, trees):
        return evaluate([check], trees)[0]["pass"]

    def test_key_presence_and_absence(self):
        trees = {"s": tree(el(key="page-Dashboard"))}
        self.assertTrue(self.passed(has_key("s", "page-Dashboard"), trees))
        self.assertFalse(self.passed(has_key("s", "page-OledCare"), trees))
        self.assertTrue(self.passed(no_key("s", "page-OledCare"), trees))
        self.assertFalse(self.passed(no_key("s", "page-Dashboard"), trees))

    def test_text_matches_by_substring_and_counts(self):
        trees = {"s": tree(el(text="Gamma Dimming 54%"), el(text="54%"), el(text="100%"))}
        self.assertTrue(self.passed(has_text("s", "Gamma Dimming 54%"), trees))
        self.assertTrue(self.passed(text_count("s", "54%", 2), trees))
        self.assertFalse(self.passed(text_count("s", "54%", 3), trees))

    def test_exactly_one_page_body(self):
        self.assertTrue(self.passed(page_keys("s"), {"s": tree(el(key="page-A"), el(key="nav-A"))}))
        both = {"s": tree(el(key="page-A"), el(key="page-B"))}
        self.assertFalse(self.passed(page_keys("s"), both))
        self.assertTrue(self.passed(page_keys("s", 2), both))

    def test_opacity_window(self):
        trees = {"s": tree(el(key="page-A", opacity=0.05))}
        self.assertTrue(self.passed(opacity_between("s", "page-A", 0.0, 0.5), trees))
        self.assertFalse(self.passed(opacity_between("s", "page-A", 0.5, 1.0), trees))
        self.assertFalse(self.passed(opacity_between("s", "missing", 0.0, 1.0), trees))

    def test_motion_between_shots(self):
        trees = {"a": tree(el(key="ring", rect=(1539, 287, 220, 220))),
                 "b": tree(el(key="ring", rect=(1604, 280, 220, 220)))}
        self.assertTrue(self.passed(moved("a", "b", "ring", 20), trees))
        self.assertFalse(self.passed(moved("a", "a", "ring", 20), trees))

    def test_missing_shot_fails_instead_of_raising(self):
        self.assertFalse(self.passed(has_key("never-captured", "k"), {}))

    def test_a_malformed_tree_is_a_failed_check_not_a_crash(self):
        bad = {"s": {"elements": [{"key": "ring"}]}}  # no rect
        result = evaluate([moved("s", "s", "ring", 1)], bad)[0]
        self.assertFalse(result["pass"])
        self.assertIn("raised", result["detail"])

    def test_text_past_the_window_or_its_card_is_reported(self):
        card = {"path": "root/card", "radius": 10.0, "rect": [100, 0, 300, 50], "text": None}
        inside = {"path": "root/card/a", "rect": [110, 5, 100, 10], "text": "fits"}
        past_card = {"path": "root/card/b", "rect": [110, 20, 320, 10], "text": "spills out of the card"}
        past_window = {"path": "root/c", "rect": [900, 0, 100, 10], "text": "off screen"}
        found = overflowing_text({"elements": [card, inside, past_card, past_window]}, 920)
        self.assertEqual([(t, where) for t, where, _ in found],
                         [("spills out of the card", "card"), ("off screen", "window")])
        self.assertTrue(evaluate([text_fits("s", 920)], {"s": {"elements": [card, inside]}})[0]["pass"])
        self.assertFalse(evaluate([text_fits("s", 920)], {"s": {"elements": [past_window]}})[0]["pass"])

    def test_an_overlay_beside_the_window_does_not_cover_it(self):
        on_screen = {"s": tree(el(key="ov", rect=(0, 0, 1160, 760)))}
        beside = {"s": tree(el(key="ov", rect=(1160, 0, 1160, 760)))}
        self.assertTrue(evaluate([covers_window("s", "ov", 1160, 760)], on_screen)[0]["pass"])
        self.assertFalse(evaluate([covers_window("s", "ov", 1160, 760)], beside)[0]["pass"])

    def test_label_centre_for_scripted_taps(self):
        t = tree(el(role="radio", label="Pixel Refresh & Shift", rect=(100, 200, 40, 20)))
        self.assertEqual(find_center(t, "Pixel Refresh & Shift"), (120.0, 210.0))
        self.assertIsNone(find_center(t, "Nope"))


class FeatureMapSyncTest(unittest.TestCase):
    """feature-map.json (what people say) and verify.py (what is checked) must not drift."""

    @classmethod
    def setUpClass(cls):
        cls.map = {f["id"]: f for f in json.loads((ROOT / "feature-map.json").read_text())["features"]}

    def test_same_feature_ids(self):
        self.assertEqual(set(self.map), set(verify.FEATURES))

    def test_verify_command_runs_that_feature(self):
        for fid, feature in self.map.items():
            self.assertEqual(feature["verify_command"], f"python3 scripts/verify.py run {fid}")

    def test_every_check_reads_a_shot_the_feature_captures(self):
        for fid, feature in verify.FEATURES.items():
            shots = [s.name for s in feature.shots]
            self.assertEqual(len(shots), len(set(shots)), f"{fid}: duplicate shot names")
            for check in feature.checks:
                self.assertTrue(set(check.shots) <= set(shots), f"{fid}: {check.name}")

    def test_every_feature_asserts_something(self):
        for fid, feature in verify.FEATURES.items():
            self.assertGreaterEqual(len(feature.checks), 3, fid)

    def test_no_feature_taps_a_hardware_control(self):
        # Only navigation and tab pickers may be driven; dimming, brightness and charge controls
        # write sysfs or the live display.
        allowed = ("nav-", "Pixel Refresh & Shift", "System & Screensaver")
        for fid, feature in verify.FEATURES.items():
            for shot in feature.shots:
                for target in (shot.tap_key, shot.tap_label):
                    self.assertTrue(target is None or target.startswith(allowed), f"{fid}: {target}")


if __name__ == "__main__":
    unittest.main()

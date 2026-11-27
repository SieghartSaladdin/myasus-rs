"""Tests for the dimming-flow model checker. Run: python3 -m unittest discover -s scripts"""

import unittest

import model_dimming


class ModelTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = model_dimming.run()
        cls.by_name = {v["name"]: v for v in cls.report["variants"]}

    def test_every_variant_behaves_as_expected(self):
        self.assertEqual(self.report["status"], "pass")

    def test_the_current_design_holds_both_properties_without_deadlock(self):
        v = self.by_name["save-then-apply under lock (current)"]
        self.assertTrue(v["S1_holds"])
        self.assertTrue(v["L_holds"])
        self.assertEqual(v["deadlocks"], 0)

    def test_the_check_still_finds_the_stale_level_race(self):
        v = self.by_name["apply-then-save, no lock (old)"]
        self.assertFalse(v["S1_holds"])
        self.assertTrue(any("guard" in step for step in v["counterexample_S1"]))
        self.assertEqual(v["counterexample_S1"][-1].split(": ")[1], "ramp=old saved=new")

    def test_the_check_still_finds_the_reset_loop(self):
        v = self.by_name["xrandr-apply (old)"]
        self.assertFalse(v["L_holds"])
        self.assertIn("csd-color", v["counterexample_L"][0])


if __name__ == "__main__":
    unittest.main()

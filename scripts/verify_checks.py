"""Pure checks over the element tree that `myasus-rs --rune --tree` writes.

No I/O and no process handling, so they unit-test against synthetic trees
(`scripts/test_verify.py`). `scripts/verify.py` runs the app and feeds them real trees.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

Trees = dict[str, dict]  # shot name -> parsed tree JSON
Outcome = tuple[bool, str]  # (passed, detail for the report)


@dataclass(frozen=True)
class Check:
    name: str
    shots: tuple[str, ...]  # which captured shots this check reads
    run: Callable[[Trees], Outcome]


def _elements(trees: Trees, shot: str) -> list[dict]:
    return trees.get(shot, {}).get("elements", [])


def _with_key(trees: Trees, shot: str, key: str) -> list[dict]:
    return [e for e in _elements(trees, shot) if e.get("key") == key]


def _with_text(trees: Trees, shot: str, text: str) -> list[dict]:
    return [e for e in _elements(trees, shot) if text in (e.get("text") or "")]


def has_key(shot: str, key: str) -> Check:
    def run(trees: Trees) -> Outcome:
        n = len(_with_key(trees, shot, key))
        return n > 0, f"{n} element(s) with key {key!r}"

    return Check(f"{shot}: key {key!r} is present", (shot,), run)


def no_key(shot: str, key: str) -> Check:
    def run(trees: Trees) -> Outcome:
        n = len(_with_key(trees, shot, key))
        return n == 0, f"{n} element(s) with key {key!r}"

    return Check(f"{shot}: key {key!r} is absent", (shot,), run)


def has_text(shot: str, text: str) -> Check:
    def run(trees: Trees) -> Outcome:
        n = len(_with_text(trees, shot, text))
        return n > 0, f"{n} element(s) containing {text!r}"

    return Check(f"{shot}: text {text!r} is shown", (shot,), run)


def text_count(shot: str, text: str, at_least: int) -> Check:
    def run(trees: Trees) -> Outcome:
        n = len(_with_text(trees, shot, text))
        return n >= at_least, f"{n} element(s) containing {text!r}, need {at_least}"

    return Check(f"{shot}: text {text!r} shown at least {at_least} times", (shot,), run)


def page_keys(shot: str, count: int = 1) -> Check:
    def run(trees: Trees) -> Outcome:
        keys = [e["key"] for e in _elements(trees, shot) if str(e.get("key", "")).startswith("page-")]
        return len(keys) == count, f"page bodies: {keys}"

    return Check(f"{shot}: exactly {count} page body", (shot,), run)


def opacity_between(shot: str, key: str, low: float, high: float) -> Check:
    def run(trees: Trees) -> Outcome:
        found = _with_key(trees, shot, key)
        if not found:
            return False, f"no element with key {key!r}"
        opacity = found[0].get("opacity")
        return opacity is not None and low <= opacity <= high, f"opacity {opacity} (want {low}..{high})"

    return Check(f"{shot}: {key!r} opacity within {low}..{high}", (shot,), run)


def moved(shot_a: str, shot_b: str, key: str, min_points: float) -> Check:
    def run(trees: Trees) -> Outcome:
        a, b = _with_key(trees, shot_a, key), _with_key(trees, shot_b, key)
        if not a or not b:
            return False, f"key {key!r} missing in {shot_a if not a else shot_b}"
        distance = max(abs(a[0]["rect"][0] - b[0]["rect"][0]), abs(a[0]["rect"][1] - b[0]["rect"][1]))
        return distance >= min_points, f"moved {distance:.1f}pt (need {min_points})"

    return Check(f"{shot_a} -> {shot_b}: {key!r} moved at least {min_points}pt", (shot_a, shot_b), run)


def find_center(tree: dict, label: str) -> tuple[float, float] | None:
    """Centre, in points, of the button or radio called `label`, for a scripted tap."""
    for e in tree.get("elements", []):
        if e.get("label") == label and e.get("role") in ("radio", "button", "tab"):
            x, y, w, h = e["rect"]
            return x + w / 2, y + h / 2
    return None


def evaluate(checks: list[Check] | tuple[Check, ...], trees: Trees) -> list[dict]:
    results = []
    for check in checks:
        try:
            passed, detail = check.run(trees)
        except Exception as exc:  # a malformed tree is a failed check, not a crashed report
            passed, detail = False, f"check raised {type(exc).__name__}: {exc}"
        results.append({"name": check.name, "pass": bool(passed), "detail": detail})
    return results

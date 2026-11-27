#!/usr/bin/env python3
"""Drive the real myasus-rs UI headlessly and check the invariants in feature-map.json.

    python3 scripts/verify.py list
    python3 scripts/verify.py run oled-care-page dashboard-page
    python3 scripts/verify.py run --all

Needs a debug build (`cargo build`): debug builds accept Rune's headless flags (`--script`,
`--tree`, `--snap`). Prints one JSON report on stdout (status, every invariant with its detail,
artifact paths); exit 0 = all passed, 1 = something failed, 2 = could not run.

Every run is sandboxed: no DISPLAY (so no xrandr or X11 gamma call can reach the real screen),
and HOME / XDG_CONFIG_HOME point at a temporary directory (so the real display.json is never read
or written). Only read-only features are driven; never add a step that taps a control that
changes brightness, dimming or other hardware state.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

from verify_checks import (Check, evaluate, find_center, has_key, has_text, moved, no_key,
                           opacity_between, page_keys, text_count, text_fits, covers_window)

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BINARY = ROOT / "target" / "debug" / "myasus-rs"
WINDOW = "1160x760"
MIN_WINDOW = "920x600"  # AppSpec.min_size: the smallest window the app allows
SETTLED = 1.2  # virtual seconds: every transition has landed
MIN_PNG_BYTES = 20_000  # a real screenshot of this UI is ~300 KB; a blank frame is a few KB
ERROR_RE = re.compile(r"panicked|^\s*(error|Error|ERROR)\b|could not be loaded", re.M)

# A complete display.json, so the app deserialises it instead of falling back to defaults.
DEFAULT_PREFS = {
    "flicker_free_dimming": 100, "pwm_safe_lock": True, "pwm_safe_floor": 55,
    "uniform_brightness_enabled": False, "uniform_brightness_cap": 75, "splendid_mode": "Normal",
    "eye_care_level": 2, "pixel_shift_enabled": True, "pixel_shift_strength": "Standard",
    "pixel_shift_interval_mins": 60, "screensaver_style": "AsusOrbit", "away_dim_enabled": False,
    "away_dim_secs": 90, "away_dim_level": 35, "screen_off_mins": 10,
}


@dataclass(frozen=True)
class Shot:
    """One headless capture: replay the optional tap and scene, stop at virtual time `at`."""

    name: str
    at: float = SETTLED
    tap_key: str | None = None  # tap the element with this key at t=0.2
    tap_label: str | None = None  # tap the button/radio with this label (found on the start page)
    scene: str | None = None
    settle: bool = True
    size: str = WINDOW


@dataclass(frozen=True)
class Feature:
    shots: tuple[Shot, ...]
    checks: tuple[Check, ...]
    prefs: dict | None = None  # overrides on DEFAULT_PREFS, written to the sandbox display.json


def page_feature(nav: str, page_key: str, texts: tuple[str, ...] = (), keys: tuple[str, ...] = ()) -> Feature:
    checks = [has_key("page", page_key), page_keys("page"), text_fits("page", 1160)]
    checks += [has_text("page", t) for t in texts] + [has_key("page", k) for k in keys]
    return Feature((Shot("page", tap_key=f"nav-{nav}"),), tuple(checks))


PAGES = (("oled", "OLED Care"), ("dashboard", "Dashboard"), ("hardware", "Hardware & Power"),
         ("diagnosis", "Diagnosis & Updates"), ("system", "System & Settings"))
FEATURES: dict[str, Feature] = {
    "oled-care-page": Feature((Shot("start"),), (
        has_key("start", "page-OledCare"), page_keys("start"), has_key("start", "nav-OLED Care"),
        has_text("start", "ASUS OLED Care (Linux Native)"), has_key("start", "oled-dim-slider"),
        text_fits("start", 1160))),
    "min-window-layout": Feature(
        tuple(Shot(n, tap_key=f"nav-{label}", size=MIN_WINDOW) for n, label in PAGES),
        tuple(text_fits(n, 920) for n, _ in PAGES)),
    "oled-sub-tabs": Feature(
        (Shot("pixel", tap_label="Pixel Refresh & Shift"), Shot("saver", tap_label="System & Screensaver")),
        (has_key("pixel", "oled-tab-1"), no_key("pixel", "oled-tab-0"),
         has_text("pixel", "Pixel Shift (Real X11 RandR Translation)"),
         has_key("saver", "oled-tab-2"), no_key("saver", "oled-tab-1"))),
    "dashboard-page": page_feature("Dashboard", "page-Dashboard", ("CPU & GPU", "Storage Overview")),
    "hardware-power-page": page_feature("Hardware & Power", "page-HardwarePower", ("Battery Health Charging",)),
    "diagnosis-updates-page": page_feature(
        "Diagnosis & Updates", "page-DiagnosisUpdates", ("Run Diagnosis", "Check Updates"), ("diag-tab-0",)),
    "system-settings-page": page_feature("System & Settings", "page-SystemSettings", ("Telemetry Refresh Interval",)),
    "page-switch-transition": Feature(
        (Shot("mid", 0.28, tap_key="nav-Dashboard", settle=False),
         Shot("late", 0.45, tap_key="nav-Dashboard", settle=False),
         Shot("end", tap_key="nav-Dashboard")),
        (has_key("mid", "page-Dashboard"), no_key("mid", "page-OledCare"), page_keys("mid"),
         opacity_between("mid", "page-Dashboard", 0.0, 0.5),
         page_keys("late"), opacity_between("late", "page-Dashboard", 0.5, 0.999),
         page_keys("end"), opacity_between("end", "page-Dashboard", 0.999, 1.0))),
    "dimming-state-display": Feature(
        (Shot("start"),),
        (text_count("start", "54%", 2), has_text("start", "Gamma Dimming 54%"), has_text("start", "OLED Guard")),
        prefs={"flicker_free_dimming": 54}),
    "oled-screensaver-preview": Feature(
        (Shot("t1", 1.0, scene="screensaver-live", settle=False), Shot("t2", 2.0, scene="screensaver-live", settle=False)),
        (has_key("t1", "oled-screensaver-root"), covers_window("t1", "oled-screensaver-root", 1160, 760),
         has_key("t1", "orbit-ring-1"),
         has_text("t1", "ASUS OLED Care • Click or press Esc to wake"), moved("t1", "t2", "orbit-ring-1", 20))),
}


def run_shot(binary: Path, home: Path, out_dir: Path, shot: Shot, prefs: dict | None,
             tap_xy: tuple[float, float] | None) -> dict:
    """Run the app once for `shot` in the sandbox and collect its tree, screenshot and stderr."""
    out_dir.mkdir(parents=True, exist_ok=True)
    config = Path(tempfile.mkdtemp(prefix="config-", dir=home))
    if prefs is not None:
        (config / "myasus-rs").mkdir()
        (config / "myasus-rs" / "display.json").write_text(json.dumps({**DEFAULT_PREFS, **prefs}))
    png, tree_path = out_dir / f"{shot.name}.png", out_dir / f"{shot.name}.tree.json"
    args = [str(binary), "--rune"]
    tap = {"key": shot.tap_key} if shot.tap_key else ({"x": tap_xy[0], "y": tap_xy[1]} if tap_xy else None)
    if tap:
        script = out_dir / f"{shot.name}.script.json"
        script.write_text(json.dumps({"events": [{"t": 0.2, "type": "tap", **tap}]}))
        args += ["--script", str(script)]
    if shot.scene:
        args += ["--scene", shot.scene]
    args += ["--at", str(shot.at)] + (["--settle"] if shot.settle else [])
    args += ["--tree", str(tree_path), "--snap", str(png), "--size", shot.size]
    env = {k: v for k, v in os.environ.items() if k not in ("DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY")}
    env.update(HOME=str(home), XDG_CONFIG_HOME=str(config), RUNE_PERF_WARN="0")
    started = time.monotonic()
    proc = subprocess.run(args, env=env, capture_output=True, text=True, timeout=240, cwd=ROOT)
    tree = json.loads(tree_path.read_text()) if tree_path.is_file() else None
    return {
        "name": shot.name, "rc": proc.returncode, "seconds": round(time.monotonic() - started, 1),
        "png": str(png.relative_to(ROOT)) if png.is_relative_to(ROOT) else str(png),
        "png_bytes": png.stat().st_size if png.is_file() else 0, "tree": tree,
        "errors": [line for line in proc.stderr.splitlines() if ERROR_RE.search(line)],
    }


def health(result: dict) -> list[dict]:
    name = result["name"]
    return [
        {"name": f"{name}: app exits cleanly", "pass": result["rc"] == 0, "detail": f"exit code {result['rc']}"},
        {"name": f"{name}: no panic or error output", "pass": not result["errors"], "detail": "; ".join(result["errors"][:3]) or "clean"},
        {"name": f"{name}: screenshot is not blank", "pass": result["png_bytes"] >= MIN_PNG_BYTES, "detail": f"{result['png_bytes']} bytes"},
    ]


def discover_labels(binary: Path, home: Path, out_root: Path, labels: set[str]) -> dict[str, tuple[float, float]]:
    """Where each label sits on the start page, so a scripted tap does not depend on pixels."""
    result = run_shot(binary, home, out_root / "_discover", Shot("start"), None, None)
    tree = result["tree"] or {}
    return {l: c for l in labels if (c := find_center(tree, l))}


def run_features(ids: list[str], binary: Path, out_root: Path, jobs: int) -> dict:
    labels = {s.tap_label for i in ids for s in FEATURES[i].shots if s.tap_label}
    with tempfile.TemporaryDirectory(prefix="myasus-verify-") as tmp:
        home = Path(tmp)
        where = discover_labels(binary, home, out_root, labels) if labels else {}
        jobs_list = [(fid, shot) for fid in ids for shot in FEATURES[fid].shots]

        def one(job: tuple[str, Shot]) -> dict:
            fid, shot = job
            return run_shot(binary, home, out_root / fid, shot, FEATURES[fid].prefs, where.get(shot.tap_label or ""))

        with ThreadPoolExecutor(max_workers=jobs) as pool:
            done = dict(zip(jobs_list, pool.map(one, jobs_list)))
    features = []
    for fid in ids:
        feature, shots = FEATURES[fid], [done[(fid, s)] for s in FEATURES[fid].shots]
        trees = {r["name"]: r["tree"] for r in shots if r["tree"]}
        missing = [s.name for s in feature.shots if s.tap_label and s.tap_label not in where]
        invariants = [{"name": f"{s}: button found on the start page", "pass": False, "detail": "label not found"} for s in missing]
        invariants += [i for r in shots for i in health(r)] + evaluate(feature.checks, trees)
        features.append({
            "id": fid, "status": "pass" if all(i["pass"] for i in invariants) else "fail",
            "invariants": invariants, "artifacts": [{"shot": r["name"], "screenshot": r["png"], "seconds": r["seconds"]} for r in shots],
        })
    failed = [f["id"] for f in features if f["status"] == "fail"]
    return {"status": "fail" if failed else "pass", "binary": str(binary), "passed": len(features) - len(failed),
            "failed": len(failed), "failed_features": failed, "features": features}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list", help="print the feature ids")
    run = sub.add_parser("run", help="verify features")
    run.add_argument("features", nargs="*")
    run.add_argument("--all", action="store_true")
    run.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    run.add_argument("--out", type=Path, default=ROOT / "artifacts" / "verify")
    run.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()

    if args.command == "list":
        print(json.dumps(sorted(FEATURES)))
        return 0
    ids = sorted(FEATURES) if args.all else args.features
    unknown = [i for i in ids if i not in FEATURES]
    if not ids or unknown:
        print(json.dumps({"status": "error", "error": f"unknown or no features: {unknown or 'none given'}", "known": sorted(FEATURES)}))
        return 2
    if not args.binary.is_file():
        print(json.dumps({"status": "error", "error": f"{args.binary} not found; run `cargo build` first"}))
        return 2
    report = run_features(ids, args.binary, args.out, args.jobs)
    print(json.dumps(report, indent=2, ensure_ascii=False))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())

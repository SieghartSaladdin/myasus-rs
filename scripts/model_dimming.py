#!/usr/bin/env python3
"""Exhaustive check of the Flicker-Free Dimming flow (see docs/dimming-flow.md).

Explores EVERY interleaving of these actors, within the bounds below, and checks two properties:

  S1 (safety)   once the user's new level has been saved and applied, the screen never shows the
                old level again.
  L  (liveness) with no further system resets, the screen settles on the saved level and stays
                there (no flapping forever).

Actors: the UI applying + saving a new level, two dimming guards (the login daemon and the app
each run one), `csd-color` (answers a RandR change by resetting the gamma ramp), and the system
(suspend/unlock/hotplug/Pixel Shift) requesting a reset.

Three variants are checked. Two are the historical implementations, kept so this check proves it
still catches the bugs we fixed; one is the current implementation.

    python3 scripts/model_dimming.py          # exit 0 when every variant behaves as expected
    python3 scripts/model_dimming.py --json

Bounds: one user change, at most 2 system resets, 2 guards. This is not TLA+ or a proof for
unbounded runs; it is exhaustive within those bounds. The model abstracts the real code: levels
are `old`/`new`/`reset`, and time is not modelled (the 3 s guard interval is not checked here).
"""

from __future__ import annotations

import json
import sys
from collections import deque
from dataclasses import dataclass, replace

OLD, NEW, RESET = "old", "new", "reset"
MAX_RESETS = 2


@dataclass(frozen=True)
class Variant:
    locked: bool  # UI change and guard check+apply each run under one cross-process lock
    apply_triggers_reset: bool  # applying gamma via `xrandr` re-sends the CRTC config
    ui_saves_first: bool  # UI persists the level before putting it on screen
    expect_s1_holds: bool
    expect_l_holds: bool


VARIANTS = {
    "xrandr-apply (old)": Variant(False, True, False, expect_s1_holds=False, expect_l_holds=False),
    "apply-then-save, no lock (old)": Variant(False, False, False, expect_s1_holds=False, expect_l_holds=True),
    "save-then-apply under lock (current)": Variant(True, False, True, expect_s1_holds=True, expect_l_holds=True),
}


@dataclass(frozen=True)
class State:
    disk: str = OLD  # level saved in display.json
    ramp: str = OLD  # level on the screen
    reset_pending: bool = False  # csd-color has a reset to do
    ui: int = 0  # 0 idle, 1 mid-change, 2 done
    lock: str | None = None
    resets_left: int = MAX_RESETS
    guards: tuple[tuple[int, str | None], ...] = ((0, None), (0, None))  # (pc, level it loaded)


def put_on_screen(s: State, level: str, v: Variant) -> State:
    return replace(s, ramp=level, reset_pending=s.reset_pending or v.apply_triggers_reset)


def steps(s: State, v: Variant):
    """Every enabled transition as (actor, next state)."""
    if s.resets_left:
        yield "system", replace(s, reset_pending=True, resets_left=s.resets_left - 1)
    if s.reset_pending:
        yield "csd-color", replace(s, ramp=RESET, reset_pending=False)

    save = lambda t: replace(t, disk=NEW)
    show = lambda t: put_on_screen(t, NEW, v)
    first, second = (save, show) if v.ui_saves_first else (show, save)
    if s.ui == 0 and (not v.locked or s.lock is None):
        yield "ui", replace(first(s), ui=1, lock="ui" if v.locked else s.lock)
    elif s.ui == 1:
        yield "ui", replace(second(s), ui=2, lock=None if v.locked else s.lock)

    for i, (pc, seen) in enumerate(s.guards):
        actor, name = f"guard{i}", f"guard{i}"
        put = lambda g, s=s: replace(s, guards=s.guards[:i] + (g,) + s.guards[i + 1:])
        if pc == 0 and (not v.locked or s.lock is None):  # load the saved level
            yield actor, replace(put((1, s.disk)), lock=name if v.locked else s.lock)
        elif pc == 1 and s.ramp == seen:  # screen already shows it: nothing to do
            yield actor, replace(put((0, None)), lock=None if v.locked else s.lock)
        elif pc == 1:  # mismatch: re-apply what was loaded
            yield actor, put((2, seen))
        elif pc == 2:
            t = put_on_screen(put((0, None)), seen, v)
            yield actor, replace(t, lock=None if v.locked else s.lock)


def explore(v: Variant):
    start = State()
    parent, edges = {start: None}, {}
    queue = deque([start])
    while queue:
        s = queue.popleft()
        edges[s] = list(steps(s, v))
        for actor, t in edges[s]:
            if t not in parent:
                parent[t] = (s, actor)
                queue.append(t)
    return parent, edges


def trace(parent, s) -> list[str]:
    out = []
    while parent[s]:
        prev, actor = parent[s]
        out.append(f"{actor}: ramp={s.ramp} saved={s.disk}")
        s = prev
    return out[::-1]


def check_s1(parent):
    bad = next((s for s in parent if s.ui == 2 and s.ramp == OLD), None)
    return bad is None, trace(parent, bad) if bad else []


def check_liveness(edges):
    """With system resets switched off, is there a fair cycle that leaves the screen wrong?"""
    graph = {s: [(a, t) for a, t in out if a != "system"] for s, out in edges.items()}
    index, low, on, stack, sccs, counter = {}, {}, set(), [], [], [0]

    def visit(root):  # iterative Tarjan
        work = [(root, iter(graph[root]))]
        index[root] = low[root] = counter[0]; counter[0] += 1
        stack.append(root); on.add(root)
        while work:
            node, it = work[-1]
            for _, nxt in it:
                if nxt not in index:
                    index[nxt] = low[nxt] = counter[0]; counter[0] += 1
                    stack.append(nxt); on.add(nxt)
                    work.append((nxt, iter(graph[nxt])))
                    break
                if nxt in on:
                    low[node] = min(low[node], index[nxt])
            else:
                work.pop()
                if work:
                    low[work[-1][0]] = min(low[work[-1][0]], low[node])
                if low[node] == index[node]:
                    comp = set()
                    while True:
                        n = stack.pop(); on.discard(n); comp.add(n)
                        if n == node:
                            break
                    sccs.append(comp)

    for s in graph:
        if s not in index:
            visit(s)
    for comp in sccs:
        inside = [(a, t) for s in comp for a, t in graph[s] if t in comp]
        if not inside:
            continue
        taken = {a for a, _ in inside}
        # actors that must keep moving: enabled somewhere here; the user need not ever act
        owed = {a for s in comp for a, _ in graph[s] if not (a == "ui" and s.ui == 0)}
        if owed <= taken:
            wrong = next((s for s in comp if s.ramp != s.disk), None)
            if wrong:
                return False, [f"can flap forever while ramp={wrong.ramp} saved={wrong.disk}; "
                               f"actors in the loop: {sorted(taken)}"]
    return True, []


def run() -> dict:
    report = {"bounds": {"user_changes": 1, "system_resets": MAX_RESETS, "guards": 2}, "variants": []}
    for name, v in VARIANTS.items():
        parent, edges = explore(v)
        s1, s1_trace = check_s1(parent)
        live, live_trace = check_liveness(edges)
        deadlock = [s for s, out in edges.items() if not out]
        report["variants"].append({
            "name": name, "states": len(parent), "deadlocks": len(deadlock),
            "S1_holds": s1, "L_holds": live,
            "as_expected": s1 == v.expect_s1_holds and live == v.expect_l_holds and not deadlock,
            "counterexample_S1": s1_trace, "counterexample_L": live_trace,
        })
    report["status"] = "pass" if all(x["as_expected"] for x in report["variants"]) else "fail"
    return report


def main() -> int:
    report = run()
    if "--json" in sys.argv:
        print(json.dumps(report, indent=2))
    else:
        b = report["bounds"]
        print(f"bounds: {b['user_changes']} user change, up to {b['system_resets']} system resets, {b['guards']} guards\n")
        for x in report["variants"]:
            mark = lambda ok: "holds" if ok else "VIOLATED"
            print(f"{x['name']}\n  states={x['states']} deadlocks={x['deadlocks']}  "
                  f"S1 {mark(x['S1_holds'])}  L {mark(x['L_holds'])}  "
                  f"{'(as expected)' if x['as_expected'] else '(UNEXPECTED)'}")
            for line in x["counterexample_S1"]:
                print("    S1 trace:", line)
            for line in x["counterexample_L"]:
                print("    L:", line)
        print("\nstatus:", report["status"])
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())

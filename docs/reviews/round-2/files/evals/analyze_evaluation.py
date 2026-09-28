#!/usr/bin/env python3
"""EP1 research analysis. No provider calls; no successor runtime implementation."""
import argparse
import collections
import copy
import json
import math
import random
from pathlib import Path

SEED = 20260928
RESAMPLES = 10_000
CATEGORIES = {"bug", "feature", "refactor", "investigation"}


def quantile(values, probability):
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (position - lower)


def metrics(totals):
    accepted_omo, accepted_rw, cost_omo, cost_rw, attempts = totals
    if not accepted_omo or not accepted_rw or cost_omo <= 0:
        raise ValueError("undefined cost-per-acceptance ratio")
    return (
        (accepted_rw - accepted_omo) / attempts,
        (cost_rw / accepted_rw) / (cost_omo / accepted_omo),
    )


def decision(delta_interval, ratio_interval):
    delta_lower = delta_interval[0]
    ratio_upper = ratio_interval[1]
    return "SURVIVES" if (
        delta_lower > -0.02
        and (ratio_upper < 0.85 or (delta_lower > 0.05 and ratio_upper < 1.10))
    ) else "WEAKENS"


def analyze(data):
    result = {"protocol": "EP1", "seed": SEED, "resamples": RESAMPLES}
    if data.get("release_blocker") is True:
        return dict(result, verdict="DIES", reason="unresolved confirmed release blocker")
    if data.get("protocol") != "EP1" or data.get("cohort_valid") is not True:
        return dict(result, verdict="WEAKENS", reason="invalid or unconfirmed EP1 cohort")
    if data.get("release_blocker") is not False:
        return dict(result, verdict="WEAKENS", reason="release-blocker status not confirmed")
    try:
        tasks = data["tasks"]
        if not isinstance(tasks, list) or len(tasks) != 120:
            raise ValueError("expected 120 tasks")
        ids, categories, groups = set(), collections.Counter(), {}
        for task in tasks:
            task_id, repo, category = task["id"], task["repo"], task["category"]
            if not isinstance(task_id, str) or not task_id or task_id in ids:
                raise ValueError("task IDs must be unique nonempty strings")
            if not isinstance(repo, str) or not repo or category not in CATEGORIES:
                raise ValueError("invalid repository or category")
            ids.add(task_id)
            categories[category] += 1
            group = groups.setdefault(repo, [])
            totals = [0, 0, 0.0, 0.0, 3]
            for index, system in enumerate(("omo", "runweft")):
                records = task[system]
                if not isinstance(records, list) or len(records) != 3:
                    raise ValueError("expected three attempts per system/task")
                for record in records:
                    accepted, cost = record["accepted"], record["api_cost_usd"]
                    if type(accepted) is not bool:
                        raise ValueError("accepted must be Boolean")
                    if type(cost) not in (int, float) or not math.isfinite(cost) or cost < 0:
                        raise ValueError("missing, nonfinite, negative or invalid API spend")
                    totals[index] += int(accepted)
                    totals[index + 2] += cost
            group.append(totals)
        if categories != collections.Counter({c: 30 for c in CATEGORIES}):
            raise ValueError("expected 30 tasks in each category")
        if len(groups) != 24 or any(len(group) != 5 for group in groups.values()):
            raise ValueError("expected 24 repositories with five tasks each")
        bundles = [tuple(map(sum, zip(*groups[repo]))) for repo in sorted(groups)]
        totals = tuple(map(sum, zip(*bundles)))
        point_delta, point_ratio = metrics(totals)
        rng = random.Random(SEED)
        deltas, ratios = [], []
        for _ in range(RESAMPLES):
            sample = [bundles[rng.randrange(24)] for _ in range(24)]
            delta, ratio = metrics(tuple(map(sum, zip(*sample))))
            deltas.append(delta)
            ratios.append(ratio)
        delta_interval = [quantile(deltas, p) for p in (0.0125, 0.9875)]
        ratio_interval = [quantile(ratios, p) for p in (0.0125, 0.9875)]
        return dict(
            result,
            verdict=decision(delta_interval, ratio_interval),
            accepted_attempts={"omo": totals[0], "runweft": totals[1]},
            api_spend_usd={"omo": totals[2], "runweft": totals[3]},
            attempts_per_system=totals[4],
            acceptance_difference=point_delta,
            cost_per_acceptance_ratio=point_ratio,
            delta_interval_97_5=delta_interval,
            ratio_interval_97_5=ratio_interval,
            note="Approximate bootstrap intervals; not benchmark evidence without audited inputs.",
        )
    except (KeyError, TypeError, ValueError, OverflowError) as error:
        return dict(result, verdict="WEAKENS", reason=str(error))


def self_test():
    categories = sorted(CATEGORIES)
    fixture = {"protocol": "EP1", "cohort_valid": True, "release_blocker": False,
               "tasks": [{"id": f"t{i:03}", "repo": f"r{i // 5:02}",
                          "category": categories[i % 4],
                          "omo": [{"accepted": True, "api_cost_usd": 1.0} for _ in range(3)],
                          "runweft": [{"accepted": True, "api_cost_usd": 0.7} for _ in range(3)]}
                         for i in range(120)]}
    answer = analyze(fixture)
    assert answer["verdict"] == "SURVIVES"
    assert answer["accepted_attempts"] == {"omo": 360, "runweft": 360}
    assert abs(answer["cost_per_acceptance_ratio"] - 0.7) < 1e-12
    assert analyze(dict(fixture, release_blocker=True, cohort_valid=False))["verdict"] == "DIES"
    assert analyze(dict(fixture, cohort_valid=False))["verdict"] == "WEAKENS"
    unknown = copy.deepcopy(fixture)
    unknown["tasks"][0]["runweft"][0]["api_cost_usd"] = None
    assert analyze(unknown)["verdict"] == "WEAKENS"
    zero = copy.deepcopy(fixture)
    for task in zero["tasks"]:
        for record in task["omo"]:
            record["accepted"] = False
    assert analyze(zero)["verdict"] == "WEAKENS"
    assert decision([-0.02, 0], [0.5, 0.6]) == "WEAKENS"
    assert decision([0, 0], [0.85, 0.85]) == "WEAKENS"
    assert decision([0.05, 0.08], [1, 1]) == "WEAKENS"
    assert decision([0.06, 0.08], [1, 1.10]) == "WEAKENS"
    assert decision([0.06, 0.08], [1, 1.09]) == "SURVIVES"
    assert quantile([0, 10], 0.25) == 2.5
    return {"status": "pass", "scope": "synthetic arithmetic and gate checks only"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", nargs="?", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        output = self_test()
    elif args.input:
        data = json.loads(args.input.read_text())
        output = analyze(data) if isinstance(data, dict) else {"verdict": "WEAKENS", "reason": "expected object"}
    else:
        parser.error("supply an EP1 input JSON file or --self-test")
    print(json.dumps(output, indent=2, allow_nan=False))

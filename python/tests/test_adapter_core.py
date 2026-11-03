from __future__ import annotations

import json
from pathlib import Path

from vfk_adapter_core import (
    ReplaceFallback,
    ReplaceUtf16,
    SurroundingSnapshot,
    plan_action,
    plan_action_with_fallback,
)
from vfk_engine import AppProfile, Commit, FieldKind, PassThrough, Preedit

ROOT = Path(__file__).resolve().parents[2]
SCENARIOS = ROOT / "adapter-conformance" / "replace.jsonl"


def _field(value: str) -> FieldKind:
    return {
        "password": FieldKind.PASSWORD,
        "number": FieldKind.NUMBER,
        "email": FieldKind.EMAIL,
        "url": FieldKind.URL,
        "terminal": FieldKind.TERMINAL,
    }.get(value, FieldKind.TEXT)


def _action(value: dict):
    kind = value.get("kind", "pass")
    if kind == "pass":
        return PassThrough()
    if kind == "preedit":
        return Preedit(value.get("text", ""), value.get("cursor", 0))
    if kind == "commit":
        return Commit(value.get("text", ""), value.get("pass_key", False))
    if kind == "replace":
        return ReplaceUtf16(
            value.get("insert", value.get("text", "")),
            value.get("delete_utf16", 0),
            value.get("pass_key", False),
        )
    raise AssertionError(f"unknown action {kind}")


def test_shared_adapter_scenarios():
    assert SCENARIOS.exists()
    count = 0
    for line in SCENARIOS.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        count += 1
        case = json.loads(line)
        raw_surrounding = case.get("surrounding")
        surrounding = None
        if raw_surrounding is not None:
            surrounding = SurroundingSnapshot(
                raw_surrounding.get("text", ""),
                raw_surrounding.get("cursor", 0),
                raw_surrounding.get("anchor", 0),
            )
        planner = plan_action_with_fallback if "fallback" in case else plan_action
        kwargs = {
            "allow_unverified_replace": case.get("allow_unverified_replace", False),
        }
        if "fallback" in case:
            kwargs["fallback"] = ReplaceFallback(case["fallback"])
        plan = planner(
            _action(case["action"]),
            _field(case.get("field", "text")),
            case.get("before", ""),
            surrounding,
            AppProfile(),
            **kwargs,
        )
        expected = case["expected"]
        assert plan.kind.value == expected["kind"], case["name"]
        assert plan.delete_utf16 == expected.get("delete_utf16", 0), case["name"]
        assert plan.delete_chars == expected.get("delete_chars", 0), case["name"]
        assert plan.pass_key == expected.get("pass_key", False), case["name"]
        if "text" in expected:
            assert plan.text == expected["text"], case["name"]
    assert count >= 10

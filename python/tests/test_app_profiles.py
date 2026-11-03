"""Hợp đồng app profile dùng chung Python ↔ Rust.

`conformance/app_profiles.json` là nguồn dữ liệu chuẩn cho alias/profile;
`conformance/app_profiles_behavior.jsonl` khóa hành vi terminal/soft-reset.
"""
from __future__ import annotations

import json
from dataclasses import asdict
from pathlib import Path

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary
from vfk_engine import Config, ImeSession, OutputMode, Ports
from vfk_engine.ports import APP_ALIASES, DEFAULT_APP_PROFILES

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "python" / "vfk_engine" / "app_profiles.json"
BEHAVIOR = ROOT / "conformance" / "app_profiles_behavior.jsonl"

_DEFAULTS = {
    "output_mode": None,
    "backspace_delay_ms": 0,
    "delay_per_char_ms": 0,
    "wait_ack": False,
    "edit_previous_word": None,
    "restore_after_space": None,
    "vietnamese": None,
    "input_method": None,
    "tone_style": None,
    "spell_check": None,
    "charset": None,
}


def _normalized(profile) -> dict:
    value = asdict(profile)
    if value["tone_style"] is not None:
        value["tone_style"] = value["tone_style"].value
    return {key: item for key, item in value.items() if item != _DEFAULTS[key]}


def test_app_profiles_contract():
    expected = json.loads(DATA.read_text(encoding="utf-8"))
    assert dict(APP_ALIASES) == expected["aliases"]
    actual = {name: _normalized(profile) for name, profile in DEFAULT_APP_PROFILES.items()}
    assert actual == expected["profiles"]


def test_terminal_profile_behavior():
    for line in BEHAVIOR.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        case = json.loads(line)
        field = FakeTextField()
        session = ImeSession(
            Ports(
                MemoryConfigStore(Config()),
                StaticAppContext(case["app"]),
                field,
                WordListDictionary(),
            ),
            OutputMode(case["mode"]),
        )
        field.type(session, case["keys"])
        assert field.text == case["text"], case["name"]

"""Chạy lại bộ conformance (oracle cho bản Rust). Nếu đổi hành vi có chủ đích:
    python -m tools.export_conformance   rồi commit lại 2 file .jsonl."""
import json
import random
from pathlib import Path

import pytest

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary
from tools.export_conformance import action_dict
from vfk_core import INPUT_METHODS, TELEX, VNI, Composer, TransformOptions, transform
from vfk_engine import Config, ImeSession, KeyEvent, OutputMode, Ports
from vi_syllable import ToneStyle

C = Path(__file__).resolve().parent.parent / "conformance"


def _opts(d):
    d = dict(d)
    d["tone_style"] = ToneStyle(d["tone_style"])
    return TransformOptions(**d)


def test_core_conformance():
    bad = []
    for line in (C / "core.jsonl").read_text(encoding="utf-8").splitlines():
        c = json.loads(line)
        got = transform(c["keys"], INPUT_METHODS[c["im"]], _opts(c["opts"]), c["final"]).text
        if got != c["expected"]:
            bad.append((c, got))
    assert not bad, bad[:5]


def test_engine_conformance():
    dic = WordListDictionary()
    bad = []
    for line in (C / "engine.jsonl").read_text(encoding="utf-8").splitlines():
        c = json.loads(line)
        cfg = dict(c["config"])
        cfg["tone_style"] = ToneStyle(cfg["tone_style"])
        field = FakeTextField(c["prefix"])
        s = ImeSession(Ports(MemoryConfigStore(Config(**cfg)), StaticAppContext("app"), field, dic),
                       OutputMode(c["mode"]))
        acts = []
        for k in c["keys"]:
            ev = KeyEvent(k)
            a = s.process_key(ev)
            field.apply(a, ev)
            acts.append(action_dict(a))
            # bất biến: màn hình khớp session với MỌI cấu hình
            shown = field.text
            assert shown.endswith(s.display), c
        if acts != c["actions"] or field.text != c["text"]:
            bad.append(c)
    assert not bad, bad[:3]


@pytest.mark.parametrize("im", [TELEX, VNI, INPUT_METHODS["simple_telex"], INPUT_METHODS["viqr"]],
                         ids=["telex", "vni", "simple_telex", "viqr"])
def test_composer_equals_transform(im):
    rnd = random.Random(5)
    alpha = {
        "telex": "aeiouydwsfrxjzqnghtcAW",
        "vni": "aeiouydnghtc0123456789",
        "simple_telex": "aeiouydwsfrxjzqnghtcAW",
        "viqr": "aeiouyd^('+~?.0AEOU",
    }[im.name]
    for _ in range(600):
        o = TransformOptions(rnd.choice(list(ToneStyle)), rnd.random() < .9)
        comp = Composer(im, o)
        keys = []
        for _ in range(rnd.randint(1, 30)):
            if keys and rnd.random() < .15:
                keys.pop()
                comp.pop()
            else:
                k = rnd.choice(alpha)
                keys.append(k)
                comp.push(k)
            for final in (False, True):
                assert comp.result(final) == transform(keys, im, o, final), keys

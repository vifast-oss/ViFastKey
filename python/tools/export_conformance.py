"""Xuất bộ test trung lập ngôn ngữ (JSON Lines) để bản Rust chạy lại và so khớp từng byte.

    python -m tools.export_conformance

Tạo:
  conformance/core.jsonl    : {"im","opts","final","keys","expected"}
  conformance/engine.jsonl  : {"mode","config","keys","actions","text"}  (actions từng phím)
Bản Python là oracle: nếu Rust khác ở bất kỳ dòng nào thì Rust sai (hoặc cập nhật cả hai).
"""
from __future__ import annotations

import json
import random
import re
import sys
from dataclasses import asdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary  # noqa: E402
from typing_variants import variants  # noqa: E402
from vfk_core import INPUT_METHODS, TransformOptions, transform  # noqa: E402
from vfk_engine import (Commit, Config, ImeSession, KeyEvent, OutputMode, PassThrough, Ports,  # noqa: E402
                        Preedit, Replace)
from vi_syllable import ToneStyle  # noqa: E402

OUT = ROOT / "conformance"
SEED = 20261029


def opts_dict(o: TransformOptions) -> dict:
    d = asdict(o)
    d["tone_style"] = o.tone_style.value
    return d


def core_cases():
    # 1) golden
    for f in sorted((ROOT / "tests/golden").glob("*.tsv")):
        for line in f.read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            spec, keys, _ = line.split("\t")[:3]
            parts = spec.split(",")
            o = TransformOptions(ToneStyle.NEW if "new" in parts else ToneStyle.OLD,
                                 "nospell" not in parts, True, "bracket" in parts, "quick" in parts)
            yield parts[0], o, "final" in parts, keys
    # 2) corpus, mọi thứ tự gõ
    words = sorted(set(re.findall(r"[^\W\d_]+", (ROOT / "tests/data/corpus_vi.txt").read_text("utf-8"))))
    for im in ("telex", "vni"):
        for w in words:
            for v in sorted(set(variants(w, INPUT_METHODS[im]))):
                for st in ToneStyle:
                    yield im, TransformOptions(tone_style=st), True, v
    # 3) ngẫu nhiên, mọi tổ hợp tuỳ chọn
    rnd = random.Random(SEED)
    alpha = {"telex": "aeiouydwsfrxjzqnghtcmpbklAEOUWDS[]", "simple_telex": "aeiouydwsfrxjzngtcAW",
             "vni": "aeiouydqngthc0123456789AEOU"}
    for _ in range(6000):
        im = rnd.choice(list(alpha))
        o = TransformOptions(rnd.choice(list(ToneStyle)), rnd.random() < .85, rnd.random() < .8,
                             rnd.random() < .2, rnd.random() < .2)
        keys = "".join(rnd.choice(alpha[im]) for _ in range(rnd.randint(1, 12)))
        if im == "vni" and keys[0].isdigit():
            keys = "a" + keys
        yield im, o, rnd.random() < .5, keys


KEYS = list("aeiouydwsfrxjzngthcAWSD ,.?") + ["Backspace"] * 3 + ["Escape", "Enter", "Left", "Shift"]


def action_dict(a) -> dict:
    if isinstance(a, PassThrough):
        return {"t": "pass"}
    if isinstance(a, Preedit):
        return {"t": "preedit", "text": a.text, "cursor": a.cursor}
    if isinstance(a, Commit):
        return {"t": "commit", "text": a.text, "pass_key": a.pass_key}
    assert isinstance(a, Replace)
    return {"t": "replace", "del": a.delete_before, "ins": a.insert, "pass_key": a.pass_key}


def config_dict(c: Config) -> dict:
    d = asdict(c)
    d["tone_style"] = c.tone_style.value
    d.pop("app_profiles")            # dùng DEFAULT_APP_PROFILES; app trong kịch bản là "app"
    return d


def engine_cases():
    rnd = random.Random(SEED + 1)
    dic = WordListDictionary()
    for _ in range(3000):
        c = Config(
            input_method=rnd.choice(["telex", "telex", "vni", "simple_telex"]),
            tone_style=rnd.choice(list(ToneStyle)),
            auto_capitalize=rnd.random() < .3, smart_caps=rnd.random() < .2,
            macros={"ko": "không", "vn": "Việt Nam"}, quick_telex=rnd.random() < .15,
            unicode_form=rnd.choice(["NFC", "NFC", "NFD"]),
            restore_after_space=rnd.random() < .7, edit_previous_word=rnd.random() < .5,
        )
        mode = rnd.choice(list(OutputMode))
        prefix = rnd.choice(["", "", "tôi học tiêng", "Việt", "hello "])
        keys = [rnd.choice(KEYS + (list("1234567890") if c.input_method == "vni" else []))
                for _ in range(rnd.randint(1, 30))]
        field = FakeTextField(prefix)
        s = ImeSession(Ports(MemoryConfigStore(c), StaticAppContext("app"), field, dic), mode)
        acts = []
        for k in keys:
            ev = KeyEvent(k)
            a = s.process_key(ev)
            field.apply(a, ev)
            acts.append(action_dict(a))
        yield {"mode": mode.value, "config": config_dict(c), "prefix": prefix, "keys": keys,
               "actions": acts, "text": field.text}


def main():
    OUT.mkdir(exist_ok=True)
    n = 0
    with open(OUT / "core.jsonl", "w", encoding="utf-8") as f:
        seen = set()
        for im, o, final, keys in core_cases():
            k = (im, tuple(sorted(opts_dict(o).items())), final, keys)
            if k in seen:
                continue
            seen.add(k)
            exp = transform(keys, INPUT_METHODS[im], o, final).text
            f.write(json.dumps({"im": im, "opts": opts_dict(o), "final": final, "keys": keys,
                                "expected": exp}, ensure_ascii=False) + "\n")
            n += 1
    m = 0
    with open(OUT / "engine.jsonl", "w", encoding="utf-8") as f:
        for case in engine_cases():
            f.write(json.dumps(case, ensure_ascii=False) + "\n")
            m += 1
    print(f"core: {n} ca, engine: {m} kịch bản")


if __name__ == "__main__":
    main()

import random
import re
import time
from pathlib import Path

import pytest

from typing_variants import variants
from vfk_core import (INPUT_METHODS, MAX_RAW, TELEX, VNI, TransformOptions, diff, rebuild, to_raw,
                      transform)
from vi_syllable import ToneStyle

HERE = Path(__file__).parent
CORPUS = sorted(set(re.findall(r"[^\W\d_]+", (HERE / "data/corpus_vi.txt").read_text(encoding="utf-8"))))


def load_golden():
    for f in sorted((HERE / "golden").glob("*.tsv")):
        for line in f.read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            opts, inp, exp = line.split("\t")[:3]
            yield pytest.param(opts, inp, exp, id=f"{f.stem}:{opts}:{inp}")


def opts_from(spec: str):
    parts = spec.split(",")
    o = TransformOptions(
        tone_style=ToneStyle.NEW if "new" in parts else ToneStyle.OLD,
        spell_check="nospell" not in parts,
        quick_telex="quick" in parts,
        bracket_keys="bracket" in parts,
    )
    return INPUT_METHODS[parts[0]], o, "final" in parts


@pytest.mark.parametrize("spec,inp,exp", list(load_golden()))
def test_golden(spec, inp, exp):
    im, o, final = opts_from(spec)
    assert transform(inp, im, o, final).text == exp


@pytest.mark.parametrize("im", [TELEX, VNI], ids=["telex", "vni"])
def test_corpus_every_typing_order(im):
    """Mọi từ trong corpus, gõ theo mọi thứ tự dấu (ngay sau chữ, dồn cuối, thanh ở bất kỳ đâu)."""
    fails = []
    for w in CORPUS:
        if "oo" in w.lower():           # xoong/boong: cần gõ 'ooo', đã có golden riêng
            continue
        for v in set(variants(w, im)):
            outs = {transform(v, im, TransformOptions(tone_style=s), final=True).text for s in ToneStyle}
            if w not in outs:
                fails.append((w, v, outs))
    assert not fails, fails[:20]


@pytest.mark.parametrize("im", [TELEX, VNI], ids=["telex", "vni"])
def test_corpus_rebuild(im):
    for w in CORPUS:
        st = ToneStyle.NEW if any(x in w.lower() for x in ("oà", "oá", "oả", "oã", "oạ", "oè", "oé", "oẻ", "oẽ", "oẹ", "uỳ", "uý", "uỷ", "uỹ", "uỵ")) else ToneStyle.OLD
        assert rebuild(w, im, TransformOptions(tone_style=st)) is not None, w


def test_english_restore_at_word_end():
    for w in "text case windows class law debug json".split():
        assert transform(w, TELEX, final=True).text == w


def test_max_raw_is_literal():
    s = "a" * (MAX_RAW + 1)
    assert transform(s, TELEX).text == s


def test_property_no_crash_and_diff_exact():
    rnd = random.Random(1)
    alphabet = "aeiouydwsfrxjzqnghtcmpbklAEOUWDS"
    for _ in range(4000):
        raw = "".join(rnd.choice(alphabet) for _ in range(rnd.randint(1, 10)))
        prev = ""
        for i in range(1, len(raw) + 1):
            for final in (False, True):
                cur = transform(raw[:i], TELEX, final=final).text
                assert cur
            cur = transform(raw[:i], TELEX).text
            e = diff(prev, cur)
            assert prev[:len(prev) - e.delete_before] + e.insert == cur
            prev = cur


def test_property_output_case_follows_input():
    rnd = random.Random(2)
    for _ in range(2000):
        raw = "".join(rnd.choice("tieengsnguowif") for _ in range(rnd.randint(1, 8)))
        assert transform(raw.upper(), TELEX).text == transform(raw, TELEX).text.upper()


def test_performance_per_key():
    words = ["nguwowif", "truwowngf", "khuyeens", "dduwowngf"] * 250
    t = time.perf_counter()
    n = 0
    for w in words:
        for i in range(1, len(w) + 1):
            transform(w[:i], TELEX)
            n += 1
    per_key_ms = (time.perf_counter() - t) / n * 1000
    assert per_key_ms < 2.0, per_key_ms      # Python; bản Rust kỳ vọng < 10 µs

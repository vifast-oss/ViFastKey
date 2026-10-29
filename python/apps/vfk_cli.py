"""vfk-cli: REPL gõ thử. Đây cũng là adapter mẫu đơn giản nhất.

Chạy:  python -m apps.vfk_cli                 (tương tác)
       python -m apps.vfk_cli --demo          (chạy ví dụ)
       python -m apps.vfk_cli --text "Tieengs Vieejt"
       python -m apps.vfk_cli --method vni --tone new --preedit --text "Vie65t"
Lệnh:  :telex :vni :simple_telex :old :new :replace :preedit :toggle :trace
       :coda_k :macro_digits :spell :help :q
       Trong dòng gõ, '<' = Backspace, '~' = Escape. '|' = Enter.
"""
from __future__ import annotations

import argparse
import sys

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary
from vfk_engine import Config, ImeSession, KeyEvent, OutputMode, Ports
from vfk_ffi import VERSION
from vi_syllable import ToneStyle

SPECIAL = {"<": "Backspace", "~": "Escape", "|": "Enter"}
DEFAULT_MACROS = {"ko": "không", "vn": "Việt Nam", "dc": "được"}


class Repl:
    def __init__(self, config: Config | None = None, mode: OutputMode = OutputMode.REPLACE,
                 trace: bool = False):
        self.store = MemoryConfigStore(config or Config(macros=dict(DEFAULT_MACROS)))
        self.mode = mode
        self.trace = trace
        self._new()

    def _new(self):
        self.field = FakeTextField()
        self.session = ImeSession(Ports(self.store, StaticAppContext("cli"), self.field, WordListDictionary()),
                                  self.mode)

    def _set(self, **kw):
        c = self.store.load()
        for k, v in kw.items():
            setattr(c, k, v)
        self.store.save(c)
        self._new()

    def command(self, cmd: str) -> bool:
        match cmd:
            case ":q" | ":quit" | ":exit":
                return False
            case ":telex" | ":vni" | ":simple_telex":
                self._set(input_method=cmd[1:])
            case ":old" | ":new":
                self._set(tone_style=ToneStyle(cmd[1:]))
            case ":replace" | ":preedit":
                self.mode = OutputMode(cmd[1:])
                self._new()
            case ":toggle":
                self.session.toggle_vietnamese()
                print("  Tiếng Việt:", "BẬT" if self.session.enabled else "TẮT")
            case ":trace":
                self.trace = not self.trace
            case ":coda_k":
                self._set(allow_coda_k=not self.store.load().allow_coda_k)
            case ":macro_digits":
                self._set(macro_digits=not self.store.load().macro_digits)
            case ":spell":
                self._set(spell_check=not self.store.load().spell_check)
            case ":help":
                print(__doc__)
            case _:
                print("  Lệnh không rõ (thử :help)")
        c = self.store.load()
        flags = []
        if c.allow_coda_k:
            flags.append("coda_k")
        if c.macro_digits:
            flags.append("macro_digits")
        if not c.spell_check:
            flags.append("no-spell")
        suffix = (" | " + ",".join(flags)) if flags else ""
        print(f"  [{c.input_method} | {c.tone_style.value} | {self.mode.value}{suffix}]")
        return True

    def type_line(self, line: str) -> str:
        self.field.committed = self.field.preedit = ""
        self.session.reset()
        for ch in line:
            ev = KeyEvent(SPECIAL.get(ch, ch))
            act = self.session.process_key(ev)
            self.field.apply(act, ev)
            if self.trace:
                print(f"  {ev.key!r:12} -> {act}  | {self.field.text!r}")
        self.field.apply(self.session.process_key(KeyEvent("Enter")))   # chốt từ cuối
        return self.field.text


DEMO = [
    "Tieengs Vieejt laf ngoon ngwx raats hay.",
    "Nguwowif Vieejt Nam ddi hojc owr truwowngf ddaij hojc",
    "hoaf thuys khoer quyeets giuwx",
    "text case windows mix six",
    "ko dc vn",
    "tieengs<g",
    "server boss data this did",
    "Thuowr nhor, xooongj, quyts, khuyur tay",
]

DEMO_VNI = "Vie65t Nam, nguo7i2 d9i"


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        prog="vfk",
        description="Gõ thử ViFastKey: chuỗi phím thô -> chữ tiếng Việt.",
    )
    ap.add_argument("--demo", action="store_true", help="chạy bộ ví dụ")
    ap.add_argument("--text", action="append", default=[], metavar="CHUỖI",
                    help="gõ một dòng rồi in kết quả (lặp lại được)")
    ap.add_argument("--method", choices=["telex", "simple_telex", "vni"], help="kiểu gõ")
    ap.add_argument("--tone", choices=["old", "new"], help="kiểu đặt dấu")
    ap.add_argument("--mode", choices=["replace", "preedit"], default="replace",
                    help="mô hình đầu ra (mặc định replace)")
    ap.add_argument("--nfd", action="store_true", help="xuất Unicode NFD")
    ap.add_argument("--no-spell", action="store_true", help="tắt kiểm tra chính tả")
    ap.add_argument("--allow-coda-k", action="store_true", help="cho phép âm cuối k (Đắk Lắk)")
    ap.add_argument("--macro-digits", action="store_true", help="macro có số (mp3, k8s)")
    ap.add_argument("--macro", action="append", default=[], metavar="KHOÁ=GIÁ TRỊ",
                    help="thêm macro (lặp lại được)")
    ap.add_argument("--trace", action="store_true", help="in từng Action")
    ap.add_argument("--version", action="version", version=f"ViFastKey {VERSION}")
    return ap


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    config = Config(macros=dict(DEFAULT_MACROS))
    if args.method:
        config.input_method = args.method
    if args.tone:
        config.tone_style = ToneStyle(args.tone)
    config.unicode_form = "NFD" if args.nfd else "NFC"
    config.spell_check = not args.no_spell
    config.allow_coda_k = args.allow_coda_k
    config.macro_digits = args.macro_digits
    for item in args.macro:
        key, sep, value = item.partition("=")
        if not sep or not key.strip():
            print(f"Macro không hợp lệ: {item!r} (cần KHOÁ=GIÁ TRỊ)", file=sys.stderr)
            return 2
        config.macros[key.strip().lower()] = value
    mode = OutputMode(args.mode)
    r = Repl(config, mode, args.trace)

    if args.demo:
        for line in DEMO:
            print(f"{line!r:60} -> {r.type_line(line)!r}")
        r.command(":vni")
        print(f"{DEMO_VNI!r:60} -> {r.type_line(DEMO_VNI)!r}")
        return 0

    if args.text:
        for line in args.text:
            print(r.type_line(line))
        return 0

    print(__doc__)
    while True:
        try:
            line = input("gõ> ")
        except EOFError:
            break
        if line.startswith(":"):
            if not r.command(line.strip()):
                break
            continue
        print("  =>", r.type_line(line))
    return 0


if __name__ == "__main__":
    sys.exit(main())

"""vfk-cli: REPL gõ thử. Đây cũng là adapter mẫu đơn giản nhất.

Chạy:  python -m apps.vfk_cli            (tương tác)
       python -m apps.vfk_cli --demo     (chạy ví dụ)
Lệnh:  :telex :vni :old :new :replace :preedit :toggle :trace :q
       Trong dòng gõ, '<' = Backspace, '~' = Escape.
"""
from __future__ import annotations

import argparse
import sys

from adapters import FakeTextField, MemoryConfigStore, StaticAppContext, WordListDictionary
from vfk_engine import Config, ImeSession, KeyEvent, OutputMode, Ports
from vi_syllable import ToneStyle

SPECIAL = {"<": "Backspace", "~": "Escape"}


class Repl:
    def __init__(self):
        self.store = MemoryConfigStore(Config(macros={"ko": "không", "vn": "Việt Nam", "dc": "được"}))
        self.mode = OutputMode.REPLACE
        self.trace = False
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
            case ":q":
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
            case _:
                print("  Lệnh không rõ")
        c = self.store.load()
        print(f"  [{c.input_method} | {c.tone_style.value} | {self.mode.value}]")
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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--demo", action="store_true")
    args = ap.parse_args()
    r = Repl()
    if args.demo:
        for line in DEMO:
            print(f"{line!r:60} -> {r.type_line(line)!r}")
        r.command(":vni")
        print(f"{'Vie65t Nam, nguo7i2 d9i':60} -> {r.type_line('Vie65t Nam, nguo7i2 d9i')!r}")
        return
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


if __name__ == "__main__":
    sys.exit(main())

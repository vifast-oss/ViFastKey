#!/usr/bin/env python3
"""IBus engine cho ViFastKey (Linux).

Chạy engine này qua component XML; phần hành vi tiếng Việt nằm trong
`adapters.ibus.IbusCore` (dùng chung engine Python). Adapter chỉ ánh xạ
IBus keyval/Action và preedit/surrounding text.
"""
from __future__ import annotations

import sys

try:
    import gi

    gi.require_version("IBus", "1.0")
    from gi.repository import IBus
except (ImportError, ValueError) as exc:  # pragma: no cover - phụ thuộc Linux
    print(f"ViFastKey IBus cần python3-gi + ibus: {exc}", file=sys.stderr)
    raise SystemExit(1)

from adapters.ibus import IbusCore  # noqa: E402
from vfk_engine import Commit, PassThrough, Preedit, Replace  # noqa: E402


SPECIAL = {
    IBus.KEY_BackSpace: "Backspace",
    IBus.KEY_Return: "Enter",
    IBus.KEY_KP_Enter: "Enter",
    IBus.KEY_Tab: "Tab",
    IBus.KEY_Escape: "Escape",
    IBus.KEY_Left: "Left",
    IBus.KEY_Right: "Right",
    IBus.KEY_Up: "Up",
    IBus.KEY_Down: "Down",
    IBus.KEY_Home: "Home",
    IBus.KEY_End: "End",
    IBus.KEY_Delete: "Delete",
    IBus.KEY_Page_Up: "PageUp",
    IBus.KEY_Page_Down: "PageDown",
}


class ViFastKeyEngine(IBus.Engine):
    def __init__(self, bus, object_path):
        super().__init__(bus, object_path)
        self.core = IbusCore(config_path="~/.config/vifastkey/config.json")

    # ---------- IBus callbacks ----------
    def do_process_key_event(self, keyval, keycode, state):
        if state & IBus.ModifierType.RELEASE_MASK:
            return False
        ctrl = bool(state & IBus.ModifierType.CONTROL_MASK)
        alt = bool(state & IBus.ModifierType.MOD1_MASK)
        meta = bool(state & (IBus.ModifierType.SUPER_MASK | IBus.ModifierType.META_MASK))
        if ctrl and not alt and not meta and keyval == IBus.KEY_F:
            self._apply(self.core.toggle())
            return True

        key = SPECIAL.get(keyval)
        if key is None:
            codepoint = IBus.keyval_to_unicode(keyval)
            if not codepoint:
                return False
            key = chr(codepoint)

        self._refresh_surrounding()
        return self._apply(self.core.process(key, ctrl=ctrl, alt=alt, meta=meta))

    def do_focus_in(self):
        try:
            app_id = self.get_program()
        except Exception:  # pragma: no cover - tùy phiên bản IBus
            app_id = None
        self._apply(self.core.focus_in(app_id))

    def do_focus_out(self):
        self._apply(self.core.focus_out())

    def do_reset(self):
        self.core.reset()
        self.hide_preedit_text()

    # ---------- helpers ----------
    def _refresh_surrounding(self):
        try:
            result = self.get_surrounding_text()
        except Exception:
            self.core.set_surrounding(None)
            return
        if not result:
            self.core.set_surrounding(None)
            return
        try:
            text_obj, cursor, _anchor = result
            text = text_obj.get_text() if hasattr(text_obj, "get_text") else str(text_obj)
            self.core.set_surrounding(text[:cursor] if cursor >= 0 else text)
        except Exception:
            self.core.set_surrounding(None)

    def _apply(self, action) -> bool:
        if isinstance(action, PassThrough):
            return False
        if isinstance(action, Preedit):
            if action.text:
                self.update_preedit_text(IBus.Text.new_from_string(action.text),
                                         action.cursor, True)
                self.show_preedit_text()
            else:
                self.hide_preedit_text()
            return True
        if isinstance(action, Commit):
            self.commit_text(IBus.Text.new_from_string(action.text))
            self.hide_preedit_text()
            return not action.pass_key
        if isinstance(action, Replace):
            if action.delete_before:
                try:
                    self.delete_surrounding_text(-action.delete_before, action.delete_before)
                except Exception:
                    # IBus/app không hỗ trợ surrounding: không thể Replace an toàn.
                    return False
            self.commit_text(IBus.Text.new_from_string(action.insert))
            self.hide_preedit_text()
            return not action.pass_key
        return False


def main() -> None:
    bus = IBus.Bus()
    if not bus.is_connected():
        print("Không kết nối được IBus daemon", file=sys.stderr)
        raise SystemExit(1)
    factory = IBus.Factory.new(bus.get_connection())
    factory.add_engine("vifastkey", ViFastKeyEngine)
    bus.request_name("org.freedesktop.IBus.ViFastKey", 0)
    IBus.main()


if __name__ == "__main__":
    main()

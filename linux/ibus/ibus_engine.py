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
from vfk_adapter_core import PlanKind, plan_action  # noqa: E402
from vfk_engine import FieldKind  # noqa: E402


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
    def __init__(self):
        # IBus.Factory khởi tạo engine không tham số; GObject.__init__ cũng
        # không nhận bus/object_path.
        super().__init__()
        self.core = IbusCore(config_path="~/.config/vifastkey/config.json")

    # ---------- IBus callbacks ----------
    def do_process_key_event(self, keyval, keycode, state):
        if state & IBus.ModifierType.RELEASE_MASK:
            return False
        self._sync_field_kind()
        ctrl = bool(state & IBus.ModifierType.CONTROL_MASK)
        shift = bool(state & IBus.ModifierType.SHIFT_MASK)
        alt = bool(state & IBus.ModifierType.MOD1_MASK)
        meta = bool(state & (IBus.ModifierType.SUPER_MASK | IBus.ModifierType.META_MASK))
        before = self.core.display
        # Alt+,: toggle nội bộ Việt/Anh; không dùng Ctrl+Space của IBus.
        if alt and not ctrl and not shift and not meta and keyval == IBUS.KEY_comma:
            self._apply(self.core.toggle(), before)
            return True

        key = SPECIAL.get(keyval)
        if key is None:
            codepoint = IBus.keyval_to_unicode(keyval)
            if not codepoint:
                return False
            # PyGObject trả str; một số binding cũ trả int.
            key = codepoint if isinstance(codepoint, str) else chr(codepoint)

        self._refresh_surrounding()
        return self._apply(self.core.process(key, ctrl=ctrl, alt=alt, meta=meta), before)

    def do_focus_in(self):
        self._sync_field_kind()
        try:
            app_id = self.get_program()
        except Exception:  # pragma: no cover - tùy phiên bản IBus
            app_id = None
        before = self.core.display
        self._apply(self.core.focus_in(app_id), before)

    def do_focus_out(self):
        before = self.core.display
        self._apply(self.core.focus_out(), before)

    def do_set_cursor_location(self, x, y, width, height):
        # IBus gọi khi caret đổi chỗ. Nếu đang soạn, đây có thể chỉ là caret
        # trong preedit; không được flush. Khi rỗng, tắt adopt từ cũ.
        if not self.core.display:
            self.core.cursor_moved()

    def do_reset(self):
        self.core.reset()
        self.hide_preedit_text()

    # ---------- helpers ----------
    def _sync_field_kind(self):
        try:
            purpose = self.get_input_purpose()
        except Exception:  # pragma: no cover - tùy phiên bản IBus
            return
        purpose_enum = getattr(IBus, "InputPurpose", None)
        if purpose_enum is None:
            return
        password = getattr(purpose_enum, "PASSWORD", None)
        url = getattr(purpose_enum, "URL", None)
        email = getattr(purpose_enum, "EMAIL", None)
        number = getattr(purpose_enum, "NUMBER", None)
        digits = getattr(purpose_enum, "DIGITS", None)
        phone = getattr(purpose_enum, "PHONE", None)
        kind = FieldKind.TEXT
        if password is not None and purpose == password:
            kind = FieldKind.PASSWORD
        elif url is not None and purpose == url:
            kind = FieldKind.URL
        elif email is not None and purpose == email:
            kind = FieldKind.EMAIL
        elif (number is not None and purpose == number) or (
                digits is not None and purpose == digits) or (
                phone is not None and purpose == phone):
            kind = FieldKind.NUMBER
        if kind != self.core.field_kind:
            before = self.core.display
            self._apply(self.core.set_field_kind(kind), before)

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
            text_obj, cursor, anchor = result
            text = text_obj.get_text() if hasattr(text_obj, "get_text") else str(text_obj)
            self.core.set_surrounding(text, cursor, anchor)
        except Exception:
            self.core.set_surrounding(None)

    def _apply(self, action, before_display: str) -> bool:
        plan = plan_action(
            action,
            self.core.field_kind,
            before_display,
            self.core.surrounding_snapshot,
            self.core.profile,
        )
        if plan.kind == PlanKind.PASS:
            if self.core.field_kind in (
                    FieldKind.PASSWORD, FieldKind.NUMBER,
                    FieldKind.EMAIL, FieldKind.URL):
                self.hide_preedit_text()
            return False
        if plan.kind == PlanKind.FAIL_SAFE:
            # Không xoá mù: chốt chữ đang soạn trước khi reset. Trong PREEDIT
            # chữ chỉ nằm ở composition; bỏ đi là mất từ của người dùng.
            text = self.core.display
            if text:
                self.commit_text(IBus.Text.new_from_string(text))
            self.core.reset()
            self.hide_preedit_text()
            # Phím gốc chưa được áp; để app xử lý tiếp (giống Fcitx5 fail-safe).
            return False
        if plan.kind == PlanKind.PREEDIT:
            if plan.text:
                self.update_preedit_text(
                    IBus.Text.new_from_string(plan.text), plan.cursor_utf16, True
                )
                self.show_preedit_text()
            else:
                self.hide_preedit_text()
            return True
        if plan.kind == PlanKind.COMMIT:
            self.commit_text(IBus.Text.new_from_string(plan.text))
            self.hide_preedit_text()
            return not plan.pass_key
        if plan.kind == PlanKind.REPLACE:
            if plan.delete_chars:
                try:
                    self.delete_surrounding_text(-plan.delete_chars, plan.delete_chars)
                except Exception:
                    # App không xoá được surrounding: giữ chữ đang soạn thay vì
                    # reset làm mất từ.
                    text = self.core.display
                    if text:
                        self.commit_text(IBus.Text.new_from_string(text))
                    self.core.reset()
                    self.hide_preedit_text()
                    return False
            self.commit_text(IBus.Text.new_from_string(plan.text))
            self.hide_preedit_text()
            return not plan.pass_key
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

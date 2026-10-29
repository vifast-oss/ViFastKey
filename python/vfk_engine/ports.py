"""Port ra (driven ports) của hexagon. Mỗi nền tảng cài đặt các Protocol này.

Rust: mỗi Protocol là 1 trait; `Ports` gom các trait object (Box<dyn ...>) hoặc generic.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol

from vi_syllable import ToneStyle


@dataclass
class AppProfile:
    """Gợi ý cho ADAPTER theo từng app (engine không dùng trực tiếp).
    Rút ra từ lỗi thực tế: vnkey (terminal, Electron), fcitx5-lotus (ack Chrome, độ trễ theo độ dài)."""
    output_mode: str | None = None       # "preedit" | "replace" | None (mặc định adapter)
    backspace_delay_ms: int = 0          # trễ cơ sở giữa các backspace giả
    delay_per_char_ms: int = 0           # cộng thêm theo số ký tự bị xoá
    wait_ack: bool = False               # chờ app xử lý xong backspace rồi mới chèn
    edit_previous_word: bool | None = None
    vietnamese: bool | None = None       # ép bật/tắt (game, vim normal mode...)


DEFAULT_APP_PROFILES: dict[str, AppProfile] = {
    # terminal: xoá + chèn tách rời dễ hỏng -> preedit commit nguyên tử
    "terminal": AppProfile(output_mode="preedit", edit_previous_word=False),
    "wezterm": AppProfile(output_mode="preedit", edit_previous_word=False),
    "kitty": AppProfile(output_mode="preedit", edit_previous_word=False),
    "alacritty": AppProfile(output_mode="preedit", edit_previous_word=False),
    "iterm2": AppProfile(output_mode="preedit", edit_previous_word=False),
    "windowsterminal": AppProfile(output_mode="preedit", edit_previous_word=False),
    # Chromium: cần chờ ack, trễ theo độ dài
    "chrome": AppProfile(wait_ack=True, delay_per_char_ms=2),
    "chromium": AppProfile(wait_ack=True, delay_per_char_ms=2),
    "brave": AppProfile(wait_ack=True, delay_per_char_ms=2),
    "msedge": AppProfile(wait_ack=True, delay_per_char_ms=2),
    # Firefox: surrounding text không tin cậy
    "firefox": AppProfile(backspace_delay_ms=4, edit_previous_word=False),
    # Electron
    "code": AppProfile(backspace_delay_ms=3),
    "slack": AppProfile(backspace_delay_ms=3),
    "discord": AppProfile(backspace_delay_ms=3),
    "notion": AppProfile(backspace_delay_ms=5),
    # game
    "minecraft": AppProfile(vietnamese=False),
}


@dataclass
class Config:
    # --- kiểu gõ
    input_method: str = "telex"             # telex | simple_telex | vni
    tone_style: ToneStyle = ToneStyle.OLD   # hòa | hoà
    spell_check: bool = True
    w_as_u: bool = True
    bracket_keys: bool = False              # [ -> ơ, ] -> ư
    quick_telex: bool = False               # cc=ch, gg=gi, ...
    # --- sau khi kết thúc từ
    auto_restore_english: bool = True       # dùng port Dictionary
    macro_enabled: bool = True
    macros: dict[str, str] = field(default_factory=dict)
    auto_capitalize: bool = False           # viết hoa đầu câu
    # --- ngữ cảnh
    excluded_apps: list[str] = field(default_factory=list)
    app_modes: dict[str, bool] = field(default_factory=dict)  # nhớ Việt/Anh theo app
    default_vietnamese: bool = True
    edit_previous_word: bool = True         # dùng port SurroundingText
    app_profiles: dict[str, AppProfile] = field(default_factory=lambda: dict(DEFAULT_APP_PROFILES))
    restore_after_space: bool = True        # Backspace ngay sau dấu cách: nạp lại trạng thái từ trước (soft reset)
    smart_caps: bool = False                # 2 chữ hoa đầu từ (trừ DD) -> gõ thẳng, bảo vệ HTTP_PORT, JSON...
    # --- đầu ra
    unicode_form: str = "NFC"               # NFC | NFD


class ConfigStore(Protocol):
    def load(self) -> Config: ...
    def save(self, config: Config) -> None: ...


class AppContext(Protocol):
    def current_app(self) -> str | None: ...


class SurroundingText(Protocol):
    def before_cursor(self, n: int) -> str | None: ...


class Dictionary(Protocol):
    def is_english(self, word: str) -> bool: ...      # phím thô, chữ thường: 'data'
    def is_vietnamese(self, word: str) -> bool: ...   # chữ tiếng Việt, thường, NFC: 'dât' 


@dataclass
class Ports:
    config: ConfigStore
    app: AppContext
    surrounding: SurroundingText
    dictionary: Dictionary


def resolve_profile(config: Config, app_id: str | None) -> AppProfile:
    """So khớp không phân biệt hoa thường, theo tên tiến trình rút gọn (bỏ .exe, đường dẫn)."""
    if not app_id:
        return AppProfile()
    key = app_id.lower().rsplit("/", 1)[-1].rsplit("\\", 1)[-1].removesuffix(".exe").removesuffix(".app")
    return config.app_profiles.get(key, AppProfile())

from adapters import FakeTextField, MemoryConfigStore
from adapters.ibus import IbusCore
from vfk_engine import Config, KeyEvent, OutputMode


def make(config=None, text="", mode=OutputMode.PREEDIT):
    field = FakeTextField(text)
    core = IbusCore(config_store=MemoryConfigStore(config or Config()), mode=mode)
    # Dùng chính ô nhập giả làm nguồn surrounding để test edit previous word.
    core.session.ports.surrounding = field
    return core, field


def type_text(core, field, text):
    for ch in text:
        ev = KeyEvent(ch)
        field.apply(core.process(ch), ev)


def test_telex_preedit_commits():
    core, field = make()
    type_text(core, field, "tieengs")
    assert field.text == "tiếng"


def test_macro_and_off_mode_core():
    core, field = make(Config(macros={"ko": "không"}, macro_in_off_mode=True,
                              default_vietnamese=False), mode=OutputMode.REPLACE)
    type_text(core, field, "ko ")
    assert field.text == "không "


def test_edit_previous_word_in_ibus_core():
    core, field = make(Config(edit_previous_word=True), text="tiêng")
    type_text(core, field, "s")
    assert field.text == "tiếng"


def test_focus_out_commits_preedit():
    core, field = make()
    type_text(core, field, "vieejt")
    act = core.focus_out()
    field.apply(act, None)
    assert field.text == "việt"


def test_reloads_config_when_file_changes(tmp_path):
    path = tmp_path / "config.json"
    path.write_text('{"input_method":"telex"}', encoding="utf-8")
    core = IbusCore(config_path=path, mode=OutputMode.REPLACE)
    assert core.session.config.input_method == "telex"

    path.write_text('{"input_method":"vni"}', encoding="utf-8")
    core.process("x")
    assert core.session.config.input_method == "vni"

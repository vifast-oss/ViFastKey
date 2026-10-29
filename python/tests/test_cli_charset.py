"""Test CLI chuyển bảng mã."""
from apps.vfk_charset import main


def test_list(capsys):
    assert main(["--list"]) == 0
    out = capsys.readouterr().out
    assert "tcvn3" in out and "viqr" in out


def test_convert_to_viqr(tmp_path):
    out = tmp_path / "viqr.txt"
    assert main(["--to", "viqr", "--text", "Tiếng Việt", "--output", str(out)]) == 0
    assert out.read_text(encoding="utf-8") == "Tie^'ng Vie^.t"


def test_convert_tcvn3_roundtrip(tmp_path):
    tcvn = tmp_path / "vi.tcvn"
    assert main(["--to", "tcvn3", "--text", "Tiếng Việt", "--output", str(tcvn)]) == 0
    assert tcvn.read_bytes().hex() == "5469d56e67205669d674"
    text = tmp_path / "vi.txt"
    assert main(["--from", "tcvn3", "--to", "unicode", "--file", str(tcvn),
                 "--output", str(text)]) == 0
    assert text.read_text(encoding="utf-8") == "Tiếng Việt"


def test_detect_file(tmp_path, capsys):
    tcvn = tmp_path / "vi.tcvn"
    assert main(["--to", "tcvn3", "--text", "Tiếng Việt", "--output", str(tcvn)]) == 0
    assert main(["--detect", "--file", str(tcvn)]) == 0
    assert "tcvn3" in capsys.readouterr().out.splitlines()

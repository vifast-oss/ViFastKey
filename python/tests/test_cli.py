"""Test CLI không tương tác (dùng cho CI và tài liệu)."""
from apps.vfk_cli import main


def test_cli_text_telex(capsys):
    assert main(["--text", "Tieengs Vieejt"]) == 0
    assert capsys.readouterr().out == "Tiếng Việt\n"


def test_cli_vni_new_tone(capsys):
    assert main(["--method", "vni", "--tone", "new", "--text", "hoa2"]) == 0
    assert capsys.readouterr().out == "hoà\n"


def test_cli_proper_name(capsys):
    assert main(["--allow-coda-k", "--text", "Ddawsk Lawsk"]) == 0
    assert capsys.readouterr().out == "Đắk Lắk\n"


def test_cli_macro_with_digits(capsys):
    args = ["--macro-digits", "--macro", "mp3=máy nghe nhạc", "--text", "mp3"]
    assert main(args) == 0
    assert capsys.readouterr().out == "máy nghe nhạc\n"


def test_cli_bad_macro(capsys):
    assert main(["--macro", "khong-co-dau-bang"]) == 2
    assert "KHOÁ=GIÁ TRỊ" in capsys.readouterr().err

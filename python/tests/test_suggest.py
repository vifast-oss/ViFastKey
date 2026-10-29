from vfk_suggest import SuggestionEngine, default_engine, suggest


def test_prefix_without_diacritics_matches_vietnamese():
    words = list(default_engine().suggest("nguoi", 20))
    assert "người" in words
    assert all("nguoi" in __import__("vfk_core").remove_diacritics(w).lower() for w in words)


def test_engine_limit_and_user_words():
    e = SuggestionEngine(["tiếng", "tiền", "tie"], limit=2)
    assert len(e.suggest("tie")) == 2
    e.add_words(["tiểu"])
    assert "tiểu" in e.suggest("tieu", 10)
    assert suggest("zzzz", 3) == []


def test_loaded_corpus_size():
    assert len(default_engine()) > 20_000

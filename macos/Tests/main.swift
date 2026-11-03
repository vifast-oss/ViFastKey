import Foundation

final class FakeTextClient: EngineTextClient {
    var committed = ""
    var preedit = ""
    var text: String { committed + preedit }

    func setMarkedText(_ text: String, selectedRange: NSRange, replacementRange: NSRange) {
        preedit = text
    }
    func insertText(_ text: String, replacementRange: NSRange) {
        committed += text
        preedit = ""
    }
    func replaceText(_ text: String, deleteBeforeUTF16 count: Int) {
        let chars = Array(committed)
        let n = min(count, chars.count)
        committed = String(chars.prefix(chars.count - n)) + text
        preedit = ""
    }

    func deleteLastUTF16() {
        guard !committed.isEmpty else { return }
        committed.removeLast()
    }
}

var failures = 0
func check(_ name: String, _ got: String, _ want: String) {
    if got == want {
        print("ok   \(name): \(got)")
    } else {
        failures += 1
        print("FAIL \(name): got \(got) want \(want)")
    }
}

func type(_ engine: VfkEngine, _ keys: String, _ client: FakeTextClient) {
    let applier = ActionApplier()
    for ch in keys {
        let action = engine.processKey(String(ch))
        _ = applier.apply(action, to: client)
    }
}

func finalize(_ engine: VfkEngine, _ client: FakeTextClient) {
    let action = engine.cursorMoved()
    _ = ActionApplier().apply(action, to: client)
}

func run(_ name: String, config: String?, keys: String, want: String) {
    let engine = VfkEngine(configJSON: config)
    let client = FakeTextClient()
    type(engine, keys, client)
    finalize(engine, client)
    check(name, client.text, want)
}

run("telex", config: nil, keys: "Tieengs Vieejt", want: "Tiếng Việt")
run("d-stroke", config: nil, keys: "vcdd ddc NDD", want: "vcđ đc NĐ")
run("english", config: nil, keys: "coffee cofffee banana", want: "cofee coffee banana")
run("open-uo", config: nil, keys: "duow thuow", want: "dươ thuơ")
run("shaped-vowel", config: nil, keys: "aw aws", want: "ă ắ")
run("vni", config: "{\"input_method\":\"vni\"}", keys: "Vie65t nam2024", want: "Việt nam2024")
run("macro", config: "{\"macros\":{\"ko\":\"không\"}}", keys: "ko ", want: "không ")
run("viqr", config: "{\"input_method\":\"viqr\"}", keys: "tie^ng'", want: "tiếng")

// Escape không được forward, text trả raw.
do {
    let engine = VfkEngine(configJSON: nil)
    let client = FakeTextClient()
    type(engine, "vieejt", client)
    let escape = engine.processKey("Escape")
    let result = ActionApplier().apply(escape, to: client)
    check("escape-text", client.text, "vieejt")
    if result.forwardKey { failures += 1; print("FAIL escape-forward: true") }
    else { print("ok   escape-forward: false") }
}

// Backspace xoá ký tự hiển thị, giữ dấu.
do {
    let engine = VfkEngine(configJSON: nil)
    let client = FakeTextClient()
    type(engine, "tieengs", client)
    let action = engine.processKey("Backspace")
    _ = ActionApplier().apply(action, to: client)
    finalize(engine, client)
    check("backspace", client.text, "tiến")
}

// App ID bundle -> preset profile
do {
    let e = VfkEngine(configJSON: nil)
    _ = e.focusChanged(appID: "com.google.Chrome")
    let p = e.appProfileJSON ?? ""
    if p.contains("\"wait_ack\":true") { print("ok   app-profile chrome") }
    else { failures += 1; print("FAIL app-profile chrome: \(p)") }
}

// Output mode: Replace không tạo marked text (không gạch chân).
do {
    let engine = VfkEngine(configJSON: nil, mode: 1)
    let client = FakeTextClient()
    let applier = ActionApplier()
    for ch in "vieejt" {
        let action = engine.processKey(String(ch))
        let result = applier.apply(action, to: client)
        // PassThrough: app thật tự chèn ký tự; fake client phải mô phỏng.
        if !result.handled {
            client.insertText(String(ch), replacementRange: NSRange(location: NSNotFound, length: 0))
        }
    }
    check("replace-mode-preedit", client.preedit, "")
    finalize(engine, client)
    check("replace-mode-text", client.text, "việt")
}

// Preedit vẫn tạo marked text khi bật.
do {
    let engine = VfkEngine(configJSON: nil, mode: 0)
    let client = FakeTextClient()
    type(engine, "vieejt", client)
    check("preedit-mode-marked", client.preedit, "việt")
    finalize(engine, client)
    check("preedit-mode-text", client.text, "việt")
}

// Soft reset: surrounding text còn stale sau khi app xoá dấu cách.
do {
    let engine = VfkEngine(configJSON: nil, mode: 1)
    let client = FakeTextClient()
    let applier = ActionApplier()
    func apply(_ key: String) {
        let action = engine.processKey(key)
        let result = applier.apply(action, to: client)
        if !result.handled {
            if key == "Backspace" { client.deleteLastUTF16() }
            else { client.insertText(key, replacementRange: NSRange(location: NSNotFound, length: 0)) }
        }
    }
    for ch in "cung " { apply(String(ch)) }
    apply("Backspace")                       // xoá dấu cách, engine khôi phục "cung"
    let action = engine.processKey("x")
    let strict = engine.planAction(action, fieldKind: 0, beforeDisplay: "cung",
                                   surroundingText: "cung ", cursorUTF16: 5, anchorUTF16: 5)
    check("soft-reset-strict", String(describing: strict.kind), "failSafe")
    let fallback = engine.planAction(action, fieldKind: 0, beforeDisplay: "cung",
                                     surroundingText: "cung ", cursorUTF16: 5, anchorUTF16: 5,
                                     replaceFallback: 1)
    check("soft-reset-fallback", String(describing: fallback.kind), "replace")
}

// Replace committed text (preedit edit-previous-word).
do {
    let client = FakeTextClient()
    client.committed = "tiêng"
    let action = VfkEngineAction(kind: .replace, deleteBeforeUTF16: 3, insert: "ếng",
                                 cursorUTF16: 0, passKey: false)
    _ = ActionApplier().apply(action, to: client)
    check("replace-committed", client.text, "tiếng")
}

// Charset clipboard helpers
do {
    let e = VfkEngine(configJSON: nil)
    check("charset-encode-viqr", e.encodeCharset("tiếng", charset: "viqr") ?? "nil", "tie^'ng")
    check("charset-decode-viqr", e.decodeCharset("tie^'ng", charset: "viqr") ?? "nil", "tiếng")
}

// Key mapping
check("key-backspace", KeyMapper.name(for: 51) ?? "nil", "Backspace")
check("key-enter", KeyMapper.name(for: 36) ?? "nil", "Enter")
check("key-escape", KeyMapper.name(for: 53) ?? "nil", "Escape")
check("key-left", KeyMapper.name(for: 123) ?? "nil", "Left")
check("key-none", KeyMapper.name(for: 0) ?? "nil", "nil")

if failures > 0 {
    print("\(failures) FAILED")
    exit(1)
}
print("all adapter self-tests passed")

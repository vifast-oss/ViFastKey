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
run("english", config: nil, keys: "coffee banana", want: "coffee banana")
run("open-uo", config: nil, keys: "duow thuow", want: "dươ thuơ")
run("shaped-vowel", config: nil, keys: "aw aws", want: "ă ắ")
run("vni", config: "{\"input_method\":\"vni\"}", keys: "Vie65t nam2024", want: "Việt nam2024")
run("macro", config: "{\"macros\":{\"ko\":\"không\"}}", keys: "ko ", want: "không ")

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

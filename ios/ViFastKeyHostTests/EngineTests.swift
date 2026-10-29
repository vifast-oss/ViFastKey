import XCTest
@testable import ViFastKeyHost

final class EngineTests: XCTestCase {
    func testTelexThroughRustEngine() {
        let engine = VfkEngine()
        var committed = ""
        var preedit = ""
        for ch in "tieengs" {
            let action = engine.processKey(String(ch))
            switch action.kind {
            case .preedit:
                preedit = action.insert
            case .commit, .replace:
                committed += action.insert
                preedit = ""
            case .pass:
                break
            }
        }
        let end = engine.processKey(" ")
        if end.kind == .commit || end.kind == .replace {
            committed += end.insert
            preedit = ""
        }
        XCTAssertEqual(committed, "tiếng ")
        XCTAssertEqual(preedit, "")
    }

    func testToggleVietnamese() {
        let engine = VfkEngine()
        XCTAssertTrue(engine.isEnabled)
        _ = engine.toggleVietnamese()
        XCTAssertFalse(engine.isEnabled)
        _ = engine.toggleVietnamese()
        XCTAssertTrue(engine.isEnabled)
    }
}

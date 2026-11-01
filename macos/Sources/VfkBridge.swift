import Foundation

// C ABI từ rust/crates/vfk-ffi/include/vifastkey.h, nạp qua Bridging.h.
// Swift chỉ chuyển Action; không chứa logic tiếng Việt.

enum VfkActionKind: UInt32 {
    case pass = 0
    case preedit = 1
    case replace = 2
    case commit = 3
}

struct VfkEngineAction {
    var kind: VfkActionKind
    var deleteBeforeUTF16: Int
    var insert: String
    var cursorUTF16: Int
    var passKey: Bool

    static let pass = VfkEngineAction(kind: .pass, deleteBeforeUTF16: 0, insert: "",
                                      cursorUTF16: 0, passKey: false)
}

enum VfkPlanKind: UInt32 {
    case pass = 0
    case preedit = 1
    case commit = 2
    case replace = 3
    case failSafe = 4
}

/// Kết quả từ adapter-core dùng chung: đã qua field policy, surrounding guard
/// và timing. Swift chỉ cần thi hành primitive tương ứng.
struct VfkEnginePlan {
    var kind: VfkPlanKind
    var deleteUTF16: Int
    var deleteChars: Int
    var cursorUTF16: Int
    var waitMs: Int
    var passKey: Bool
    var text: String

    static let pass = VfkEnginePlan(kind: .pass, deleteUTF16: 0, deleteChars: 0,
                                    cursorUTF16: 0, waitMs: 0, passKey: false, text: "")
}

final class VfkEngine {
    private var handle: OpaquePointer?

    /// `mode`: 0 = PREEDIT (marked text, có gạch chân), 1 = REPLACE (commit thẳng).
    init(configJSON: String? = nil, mode: UInt32 = 0) {
        if let json = configJSON {
            handle = json.withCString { vfk_session_new($0, mode) }
        } else {
            handle = vfk_session_new(nil, mode)
        }
        if handle == nil {
            handle = vfk_session_new(nil, mode)
        }
    }

    deinit {
        if let h = handle {
            vfk_session_free(h)
        }
    }

    var isOpen: Bool { handle != nil }

    private func convert(_ raw: VfkAction) -> VfkEngineAction {
        let text: String
        if let ptr = raw.text {
            text = String(cString: ptr)
        } else {
            text = ""
        }
        return VfkEngineAction(
            kind: VfkActionKind(rawValue: raw.kind) ?? .pass,
            deleteBeforeUTF16: Int(raw.delete_utf16),
            insert: text,
            cursorUTF16: Int(raw.cursor_utf16),
            passKey: raw.pass_key
        )
    }

    func processKey(_ key: String, ctrl: Bool = false, alt: Bool = false,
                    meta: Bool = false) -> VfkEngineAction {
        guard let h = handle else { return .pass }
        let raw = key.withCString { vfk_process_key(h, $0, ctrl, alt, meta) }
        return convert(raw)
    }

    var displayText: String {
        guard let h = handle, let ptr = vfk_display_text(h) else { return "" }
        return String(cString: ptr)
    }

    /// Lập kế hoạch bằng adapter-core dùng chung (field policy + surrounding
    /// guard + timing). Port chỉ thi hành plan.
    func planAction(_ action: VfkEngineAction,
                    fieldKind: UInt32,
                    beforeDisplay: String,
                    surroundingText: String?,
                    cursorUTF16: UInt32,
                    anchorUTF16: UInt32,
                    allowUnverifiedReplace: Bool = false) -> VfkEnginePlan {
        guard let h = handle else { return .pass }
        var raw = VfkAction()
        raw.kind = action.kind.rawValue
        raw.delete_utf16 = UInt32(max(0, action.deleteBeforeUTF16))
        raw.cursor_utf16 = UInt32(max(0, action.cursorUTF16))
        raw.pass_key = action.passKey
        var out = VfkPlan()
        let ok = action.insert.withCString { textPtr -> Bool in
            raw.text = textPtr
            return beforeDisplay.withCString { beforePtr in
                let beforeArg: UnsafePointer<CChar>? =
                    beforeDisplay.isEmpty ? nil : beforePtr
                if let surroundingText = surroundingText {
                    return surroundingText.withCString { surroundingPtr in
                        vfk_plan_action(h, &raw, fieldKind, beforeArg,
                                        surroundingPtr, cursorUTF16, anchorUTF16,
                                        allowUnverifiedReplace, &out)
                    }
                }
                return vfk_plan_action(h, &raw, fieldKind, beforeArg, nil,
                                       cursorUTF16, anchorUTF16,
                                       allowUnverifiedReplace, &out)
            }
        }
        guard ok else { return .pass }
        return VfkEnginePlan(
            kind: VfkPlanKind(rawValue: out.kind) ?? .pass,
            deleteUTF16: Int(out.delete_utf16),
            deleteChars: Int(out.delete_chars),
            cursorUTF16: Int(out.cursor_utf16),
            waitMs: Int(out.wait_ms),
            passKey: out.pass_key,
            text: out.text.map { String(cString: $0) } ?? ""
        )
    }

    func setSurroundingText(_ text: String?) {
        guard let h = handle else { return }
        if let text {
            text.withCString { vfk_set_surrounding_text(h, $0) }
        } else {
            vfk_set_surrounding_text(h, nil)
        }
    }

    func focusChanged(appID: String?) -> VfkEngineAction {
        guard let h = handle else { return .pass }
        let raw: VfkAction
        if let appID {
            raw = appID.withCString { vfk_focus_changed(h, $0) }
        } else {
            raw = vfk_focus_changed(h, nil)
        }
        return convert(raw)
    }

    func cursorMoved() -> VfkEngineAction {
        guard let h = handle else { return .pass }
        return convert(vfk_cursor_moved(h))
    }


    /// 0 = none, 1 = PREEDIT, 2 = REPLACE (từ AppProfile).
    var profileOutputMode: UInt32 {
        guard let h = handle else { return 0 }
        return vfk_profile_output_mode(h)
    }

    /// 0 = PREEDIT, 1 = REPLACE.
    func setOutputMode(_ mode: UInt32) -> VfkEngineAction {
        guard let h = handle else { return .pass }
        return convert(vfk_set_output_mode(h, mode))
    }

    func toggleVietnamese() -> VfkEngineAction {
        guard let h = handle else { return .pass }
        return convert(vfk_toggle_vietnamese(h))
    }

    func reset() {
        if let h = handle { vfk_reset(h) }
    }

    var isEnabled: Bool {
        guard let h = handle else { return false }
        return vfk_is_enabled(h)
    }

    var configJSON: String? {
        guard let h = handle, let ptr = vfk_config_json(h) else { return nil }
        return String(cString: ptr)
    }

    var appProfileJSON: String? {
        guard let h = handle, let ptr = vfk_app_profile_json(h) else { return nil }
        return String(cString: ptr)
    }

    @discardableResult
    func setConfigJSON(_ json: String) -> Bool {
        guard let h = handle else { return false }
        return json.withCString { vfk_set_config_json(h, $0) }
    }

    func charsetList() -> String {
        guard let ptr = vfk_charset_list() else { return "" }
        return String(cString: ptr)
    }

    func encodeCharset(_ text: String, charset: String) -> String? {
        guard let h = handle else { return nil }
        return charset.withCString { cs in
            text.withCString { txt in
                guard let ptr = vfk_charset_encode(h, cs, txt) else { return nil }
                return String(cString: ptr)
            }
        }
    }

    func decodeCharset(_ text: String, charset: String) -> String? {
        guard let h = handle else { return nil }
        return charset.withCString { cs in
            text.withCString { txt in
                guard let ptr = vfk_charset_decode(h, cs, txt) else { return nil }
                return String(cString: ptr)
            }
        }
    }
}

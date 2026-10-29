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

final class VfkEngine {
    private var handle: OpaquePointer?

    init(configJSON: String? = nil) {
        if let json = configJSON {
            handle = json.withCString { vfk_session_new($0, 0) }
        } else {
            handle = vfk_session_new(nil, 0)
        }
        if handle == nil {
            handle = vfk_session_new(nil, 0)
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

    func setFieldKind(_ kind: UInt32) {
        guard let h = handle else { return }
        _ = vfk_set_field_kind(h, kind)
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

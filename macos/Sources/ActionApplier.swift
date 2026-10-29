import Foundation

protocol EngineTextClient: AnyObject {
    func setMarkedText(_ text: String, selectedRange: NSRange, replacementRange: NSRange)
    func insertText(_ text: String, replacementRange: NSRange)
    /// Thay thế `deleteBeforeUTF16` code unit ngay trước con trỏ (dùng cho preedit
    /// sửa từ đã commit, Action Replace trong PREEDIT).
    func replaceText(_ text: String, deleteBeforeUTF16: Int)
}

struct ApplyResult {
    let handled: Bool
    let forwardKey: Bool
}

extension ActionApplier {
    /// Thi hành plan đã qua adapter-core (guard/timing/field policy).
    func apply(_ plan: VfkEnginePlan, to client: EngineTextClient) -> ApplyResult {
        switch plan.kind {
        case .pass, .failSafe:
            return ApplyResult(handled: false, forwardKey: true)
        case .preedit:
            let len = (plan.text as NSString).length
            let cursor = max(0, min(plan.cursorUTF16, len))
            client.setMarkedText(
                plan.text,
                selectedRange: NSRange(location: cursor, length: 0),
                replacementRange: NSRange(location: NSNotFound, length: 0)
            )
            return ApplyResult(handled: true, forwardKey: false)
        case .commit:
            client.insertText(plan.text, replacementRange: NSRange(location: NSNotFound, length: 0))
            return ApplyResult(handled: true, forwardKey: plan.passKey)
        case .replace:
            client.replaceText(plan.text, deleteBeforeUTF16: plan.deleteUTF16)
            return ApplyResult(handled: true, forwardKey: plan.passKey)
        }
    }
}

/// Chỉ áp Action lên client; không có logic tiếng Việt.
struct ActionApplier {
    func apply(_ action: VfkEngineAction, to client: EngineTextClient) -> ApplyResult {
        switch action.kind {
        case .pass:
            return ApplyResult(handled: false, forwardKey: true)
        case .preedit:
            let len = (action.insert as NSString).length
            let cursor = max(0, min(action.cursorUTF16, len))
            client.setMarkedText(
                action.insert,
                selectedRange: NSRange(location: cursor, length: 0),
                replacementRange: NSRange(location: NSNotFound, length: 0)
            )
            return ApplyResult(handled: true, forwardKey: false)
        case .commit:
            client.insertText(action.insert, replacementRange: NSRange(location: NSNotFound, length: 0))
            return ApplyResult(handled: true, forwardKey: action.passKey)
        case .replace:
            client.replaceText(action.insert, deleteBeforeUTF16: action.deleteBeforeUTF16)
            return ApplyResult(handled: true, forwardKey: action.passKey)
        }
    }
}

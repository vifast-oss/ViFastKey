import Foundation
import InputMethodKit

/// Bọc IMKTextInput thành protocol nhỏ để test bằng fake client.
final class ImkTextClient: EngineTextClient {
    private weak var client: AnyObject?

    init(_ client: AnyObject) {
        self.client = client
    }

    func setMarkedText(_ text: String, selectedRange: NSRange, replacementRange: NSRange) {
        guard let c = client as? IMKTextInput else { return }
        c.setMarkedText(text, selectionRange: selectedRange, replacementRange: replacementRange)
    }

    func insertText(_ text: String, replacementRange: NSRange) {
        guard let c = client as? IMKTextInput else { return }
        c.insertText(text, replacementRange: replacementRange)
    }

    func beforeCursor(_ maxLength: Int) -> String? {
        guard let c = client as? IMKTextInput else { return nil }
        let selected = c.selectedRange()
        guard selected.location != NSNotFound else { return nil }
        let start = max(0, selected.location - maxLength)
        let length = selected.location - start
        guard length > 0 else { return "" }
        return c.attributedSubstring(from: NSRange(location: start, length: length))?.string
    }

    func replaceText(_ text: String, deleteBeforeUTF16 count: Int) {
        guard let c = client as? IMKTextInput else { return }
        let selected = c.selectedRange()
        let location = max(0, selected.location - max(0, count))
        let length = min(max(0, count), selected.location)
        c.insertText(text, replacementRange: NSRange(location: location, length: length))
    }

    /// Snapshot đầy đủ cho adapter-core: text + cursor/anchor UTF-16 + 1 ký tự
    /// sau cursor để phát hiện caret giữa từ.
    func surroundingSnapshot(maxLength: Int) -> (text: String, cursorUTF16: UInt32, anchorUTF16: UInt32)? {
        guard let c = client as? IMKTextInput else { return nil }
        let selected = c.selectedRange()
        guard selected.location != NSNotFound else { return nil }

        let start = max(0, selected.location - maxLength)
        let beforeLength = max(0, selected.location - start)
        let before = beforeLength > 0
            ? c.attributedSubstring(from: NSRange(location: start, length: beforeLength))?.string ?? ""
            : ""
        let selectedText = selected.length > 0
            ? c.attributedSubstring(from: selected)?.string ?? ""
            : ""
        let afterLocation = selected.location + selected.length
        let after = c.attributedSubstring(from: NSRange(location: afterLocation, length: 1))?.string ?? ""

        let text = before + selectedText + after
        let cursor = (before as NSString).length + (selectedText as NSString).length
        let anchor = (before as NSString).length
        return (text, UInt32(cursor), UInt32(anchor))
    }
}

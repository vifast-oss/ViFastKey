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
}

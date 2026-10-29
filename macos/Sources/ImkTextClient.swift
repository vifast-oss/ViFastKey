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
}

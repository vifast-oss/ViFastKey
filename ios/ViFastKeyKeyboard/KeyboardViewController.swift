import UIKit

final class KeyboardViewController: UIInputViewController {
    private let engine = VfkEngine()
    private var keyboardView: UIStackView!

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = UIColor(white: 0.88, alpha: 1)
        buildKeyboard()
        _ = engine.focusChanged(appID: nil)
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        _ = engine.focusChanged(appID: nil)
    }

    override func textWillChange(_ textInput: UITextInput?) {
        super.textWillChange(textInput)
        engine.setSurroundingText(textDocumentProxy.documentContextBeforeInput)
    }

    private func buildKeyboard() {
        let root = UIStackView()
        root.axis = .vertical
        root.spacing = 5
        root.distribution = .fillEqually
        root.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(root)
        NSLayoutConstraint.activate([
            root.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 4),
            root.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -4),
            root.topAnchor.constraint(equalTo: view.topAnchor, constant: 6),
            root.bottomAnchor.constraint(equalTo: view.bottomAnchor, constant: -6),
        ])
        addRow(root, ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"])
        addRow(root, ["a", "s", "d", "f", "g", "h", "j", "k", "l"])
        addRow(root, ["z", "x", "c", "v", "b", "n", "m"])
        addRow(root, ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"])
        addRow(root, ["VI", " ", "Backspace", "Enter"], widths: [0.8, 3.2, 1.2, 1.2])
        keyboardView = root
    }

    private func addRow(_ root: UIStackView, _ keys: [String], widths: [CGFloat]? = nil) {
        let row = UIStackView()
        row.axis = .horizontal
        row.spacing = 4
        row.distribution = .fill
        for (index, key) in keys.enumerated() {
            let button = UIButton(type: .system)
            button.setTitle(label(for: key), for: .normal)
            button.setTitleColor(.label, for: .normal)
            button.titleLabel?.font = .systemFont(ofSize: 20)
            button.backgroundColor = .systemBackground
            button.layer.cornerRadius = 5
            button.accessibilityIdentifier = key
            button.addTarget(self, action: #selector(keyTapped(_:)), for: .touchUpInside)
            let width = widths?[index] ?? 1
            row.addArrangedSubview(button)
            if widths == nil {
                button.widthAnchor.constraint(equalTo: row.widthAnchor, multiplier: 1 / CGFloat(keys.count)).isActive = true
            } else {
                button.widthAnchor.constraint(equalTo: row.widthAnchor, multiplier: width / widths!.reduce(0, +)).isActive = true
            }
        }
        root.addArrangedSubview(row)
    }

    private func label(for key: String) -> String {
        switch key {
        case " ": return "␣"
        case "Backspace": return "⌫"
        case "Enter": return "⏎"
        default: return key
        }
    }

    @objc private func keyTapped(_ sender: UIButton) {
        guard let key = sender.accessibilityIdentifier else { return }
        textDocumentProxy.documentContextBeforeInput.map { engine.setSurroundingText($0) }
        if key == "VI" {
            apply(engine.toggleVietnamese(), original: "")
            return
        }
        apply(engine.processKey(key), original: key)
    }

    private func apply(_ action: VfkEngineAction, original: String) {
        switch action.kind {
        case .pass:
            insertOriginal(original)
        case .preedit:
            if action.insert.isEmpty {
                textDocumentProxy.unmarkText()
            } else {
                textDocumentProxy.setMarkedText(
                    action.insert,
                    selectedRange: NSRange(location: action.cursorUTF16, length: 0)
                )
            }
        case .commit:
            textDocumentProxy.insertText(action.insert)
            if action.passKey {
                insertOriginal(original)
            }
        case .replace:
            for _ in 0..<action.deleteBeforeUTF16 {
                textDocumentProxy.deleteBackward()
            }
            textDocumentProxy.insertText(action.insert)
            if action.passKey {
                insertOriginal(original)
            }
        }
    }

    private func insertOriginal(_ key: String) {
        switch key {
        case "Backspace":
            textDocumentProxy.deleteBackward()
        case "Enter":
            textDocumentProxy.insertText("\n")
        case "Tab":
            textDocumentProxy.insertText("\t")
        case "":
            break
        default:
            textDocumentProxy.insertText(key)
        }
    }
}

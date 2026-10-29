import Cocoa

extension Notification.Name {
    static let viFastKeyConfigChanged = Notification.Name("ViFastKeyConfigChanged")
}

/// Cửa sổ cấu hình tối giản (AppKit, không cần Xcode project).
final class PreferencesWindowController: NSWindowController {
    static let shared = PreferencesWindowController()

    private var dict: [String: Any] = [:]
    private var methodPopup = NSPopUpButton()
    private var tonePopup = NSPopUpButton()
    private var checks: [String: NSButton] = [:]
    private var macroView = NSTextView()

    private static let methods: [(String, String)] = [
        ("telex", "Telex"), ("simple_telex", "Simple Telex"), ("vni", "VNI"), ("viqr", "VIQR"),
    ]
    private static let boolKeys = [
        "spell_check", "w_as_u", "bracket_keys", "quick_telex", "allow_coda_k",
        "macro_digits", "macro_undo", "macro_in_off_mode", "auto_capitalize",
        "smart_caps", "edit_previous_word",
    ]

    init() {
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 520, height: 560),
                              styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "ViFastKey — Cấu hình"
        window.isReleasedWhenClosed = false
        super.init(window: window)
        buildUI()
        loadValues()
        window.center()
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) chưa hỗ trợ") }

    private func buildUI() {
        guard let content = window?.contentView else { return }
        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 18),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -18),
            stack.topAnchor.constraint(equalTo: content.topAnchor, constant: 16),
        ])

        let methodRow = row("Kiểu gõ", methodPopup)
        methodPopup.addItems(withTitles: Self.methods.map(\.1))
        stack.addArrangedSubview(methodRow)
        let toneRow = row("Đặt dấu", tonePopup)
        tonePopup.addItems(withTitles: ["Cũ (hòa)", "Mới (hoà)"])
        stack.addArrangedSubview(toneRow)

        for key in Self.boolKeys {
            let check = NSButton(checkboxWithTitle: label(for: key), target: nil, action: nil)
            checks[key] = check
            stack.addArrangedSubview(check)
        }

        let macroLabel = NSTextField(labelWithString: "Macro (mỗi dòng key=giá trị):")
        macroLabel.font = .boldSystemFont(ofSize: 12)
        stack.addArrangedSubview(macroLabel)
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.borderType = .bezelBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.heightAnchor.constraint(equalToConstant: 92).isActive = true
        macroView.isEditable = true
        macroView.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        scroll.documentView = macroView
        stack.addArrangedSubview(scroll)

        let buttons = NSStackView()
        buttons.orientation = .horizontal
        buttons.spacing = 8
        let save = NSButton(title: "Lưu", target: self, action: #selector(save))
        save.keyEquivalent = "\r"
        let cancel = NSButton(title: "Huỷ", target: self, action: #selector(cancel))
        let open = NSButton(title: "Mở file JSON", target: self, action: #selector(openJSON))
        buttons.addArrangedSubview(save)
        buttons.addArrangedSubview(cancel)
        buttons.addArrangedSubview(open)
        stack.addArrangedSubview(buttons)
    }

    private func row(_ title: String, _ control: NSView) -> NSStackView {
        let label = NSTextField(labelWithString: title)
        label.widthAnchor.constraint(equalToConstant: 110).isActive = true
        let row = NSStackView(views: [label, control])
        row.orientation = .horizontal
        row.spacing = 8
        return row
    }

    private func label(for key: String) -> String {
        switch key {
        case "spell_check": return "Kiểm tra chính tả"
        case "w_as_u": return "w → ư"
        case "bracket_keys": return "[ ] → ơ ư"
        case "quick_telex": return "Quick Telex"
        case "allow_coda_k": return "Âm cuối k (Đắk Lắk)"
        case "macro_digits": return "Macro có số (mp3, k8s)"
        case "macro_undo": return "Backspace hoàn tác macro"
        case "macro_in_off_mode": return "Macro khi tắt tiếng Việt"
        case "auto_capitalize": return "Viết hoa đầu câu"
        case "smart_caps": return "Bảo vệ chữ HOA"
        case "edit_previous_word": return "Sửa từ vừa gõ"
        default: return key
        }
    }

    private func loadValues() {
        ConfigFile.ensureExists()
        dict = ConfigFile.loadDictionary()
        let method = dict["input_method"] as? String ?? "telex"
        if let index = Self.methods.firstIndex(where: { $0.0 == method }) {
            methodPopup.selectItem(at: index)
        }
        tonePopup.selectItem(at: (dict["tone_style"] as? String == "new") ? 1 : 0)
        for (key, check) in checks {
            check.state = (dict[key] as? Bool ?? false) ? .on : .off
        }
        let macros = dict["macros"] as? [String: String] ?? [:]
        macroView.string = macros.map { "\($0.key)=\($0.value)" }.sorted().joined(separator: "\n")
    }

    @objc private func save() {
        dict["input_method"] = Self.methods[methodPopup.indexOfSelectedItem].0
        dict["tone_style"] = tonePopup.indexOfSelectedItem == 1 ? "new" : "old"
        for (key, check) in checks {
            dict[key] = check.state == .on
        }
        var macros: [String: String] = [:]
        for line in macroView.string.split(separator: "\n") {
            let parts = line.split(separator: "=", maxSplits: 1).map(String.init)
            if parts.count == 2, !parts[0].trimmingCharacters(in: .whitespaces).isEmpty {
                macros[parts[0].trimmingCharacters(in: .whitespaces).lowercased()] = parts[1]
            }
        }
        dict["macros"] = macros
        ConfigFile.saveDictionary(dict)
        NotificationCenter.default.post(name: .viFastKeyConfigChanged, object: nil)
        window?.orderOut(nil)
    }

    @objc private func cancel() {
        window?.orderOut(nil)
    }

    @objc private func openJSON() {
        ConfigFile.ensureExists()
        NSWorkspace.shared.open(ConfigFile.path)
    }

    func show() {
        loadValues()
        showWindow(nil)
        NSApp.activate(ignoringOtherApps: true)
    }
}

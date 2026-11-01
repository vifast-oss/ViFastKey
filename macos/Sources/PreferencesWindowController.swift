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
    private var unicodePopup = NSPopUpButton()
    private var excludedField = NSTextField()
    private var checks: [String: NSButton] = [:]
    private var macroView = NSTextView()

    private static let methods: [(String, String)] = [
        ("telex", "Telex"), ("simple_telex", "Simple Telex"), ("vni", "VNI"), ("viqr", "VIQR"),
    ]
    private static let boolKeys = [
        "spell_check", "w_as_u", "bracket_keys", "quick_telex", "allow_coda_k",
        "macro_enabled", "macro_digits", "macro_undo",
        "macro_in_off_mode", "auto_capitalize", "smart_caps", "edit_previous_word",
        "default_vietnamese", "restore_after_space",
    ]

    init() {
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 540, height: 680),
                              styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
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

        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        scroll.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(scroll)
        NSLayoutConstraint.activate([
            scroll.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            scroll.topAnchor.constraint(equalTo: content.topAnchor),
            scroll.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        ])

        let document = NSView()
        document.translatesAutoresizingMaskIntoConstraints = false
        scroll.documentView = document

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false
        document.addSubview(stack)
        NSLayoutConstraint.activate([
            document.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor),
            stack.leadingAnchor.constraint(equalTo: document.leadingAnchor, constant: 18),
            stack.trailingAnchor.constraint(equalTo: document.trailingAnchor, constant: -18),
            stack.topAnchor.constraint(equalTo: document.topAnchor, constant: 16),
            stack.bottomAnchor.constraint(equalTo: document.bottomAnchor, constant: -16),
        ])

        let methodRow = row("Kiểu gõ", methodPopup)
        methodPopup.addItems(withTitles: Self.methods.map(\.1))
        stack.addArrangedSubview(methodRow)
        let toneRow = row("Đặt dấu", tonePopup)
        tonePopup.addItems(withTitles: ["Cũ (hòa)", "Mới (hoà)"])
        stack.addArrangedSubview(toneRow)
        let unicodeRow = row("Unicode", unicodePopup)
        unicodePopup.addItems(withTitles: ["NFC (dựng sẵn)", "NFD (tổ hợp)"])
        stack.addArrangedSubview(unicodeRow)

        for key in Self.boolKeys {
            let check = NSButton(checkboxWithTitle: label(for: key), target: nil, action: nil)
            checks[key] = check
            stack.addArrangedSubview(check)
        }

        excludedField.placeholderString = "minecraft, game…"
        excludedField.widthAnchor.constraint(equalToConstant: 340).isActive = true
        stack.addArrangedSubview(row("App loại trừ", excludedField))

        let advanced = NSTextField(labelWithString: "App profile/charset nâng cao: dùng “Mở file JSON”.")
        advanced.font = .systemFont(ofSize: 11)
        advanced.textColor = .secondaryLabelColor
        stack.addArrangedSubview(advanced)

        let macroLabel = NSTextField(labelWithString: "Macro (mỗi dòng key=giá trị):")
        macroLabel.font = .boldSystemFont(ofSize: 12)
        stack.addArrangedSubview(macroLabel)
        let macroScroll = NSScrollView()
        macroScroll.hasVerticalScroller = true
        macroScroll.borderType = .bezelBorder
        macroScroll.translatesAutoresizingMaskIntoConstraints = false
        macroScroll.heightAnchor.constraint(equalToConstant: 92).isActive = true
        macroView.isEditable = true
        macroView.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        macroScroll.documentView = macroView
        stack.addArrangedSubview(macroScroll)

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

    private static func defaultBool(for key: String) -> Bool {
        switch key {
        case "spell_check", "w_as_u", "macro_enabled",
             "macro_undo", "default_vietnamese", "edit_previous_word", "restore_after_space":
            return true
        default:
            return false
        }
    }

    private func label(for key: String) -> String {
        switch key {
        case "spell_check": return "Kiểm tra chính tả"
        case "w_as_u": return "w → ư"
        case "bracket_keys": return "[ ] → ơ ư"
        case "quick_telex": return "Quick Telex"
        case "allow_coda_k": return "Âm cuối k (Đắk Lắk)"
        case "macro_enabled": return "Bật macro"
        case "macro_digits": return "Macro có số (mp3, k8s)"
        case "macro_undo": return "Backspace hoàn tác macro"
        case "macro_in_off_mode": return "Macro khi tắt tiếng Việt"
        case "auto_capitalize": return "Viết hoa đầu câu"
        case "smart_caps": return "Bảo vệ chữ HOA"
        case "edit_previous_word": return "Sửa từ vừa gõ"
        case "default_vietnamese": return "Mặc định tiếng Việt"
        case "restore_after_space": return "Backspace sau dấu cách nạp lại từ trước"
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
        tonePopup.selectItem(at: (dict["tone_style"] as? String == "old") ? 0 : 1)
        unicodePopup.selectItem(at: (dict["unicode_form"] as? String == "NFD") ? 1 : 0)
        for (key, check) in checks {
            check.state = (dict[key] as? Bool ?? Self.defaultBool(for: key)) ? .on : .off
        }
        excludedField.stringValue = (dict["excluded_apps"] as? [String] ?? []).joined(separator: ", ")
        let macros = dict["macros"] as? [String: String] ?? [:]
        macroView.string = macros.map { "\($0.key)=\($0.value)" }.sorted().joined(separator: "\n")
    }

    @objc private func save() {
        dict["input_method"] = Self.methods[methodPopup.indexOfSelectedItem].0
        dict["tone_style"] = tonePopup.indexOfSelectedItem == 1 ? "new" : "old"
        dict["unicode_form"] = unicodePopup.indexOfSelectedItem == 1 ? "NFD" : "NFC"
        for (key, check) in checks {
            dict[key] = check.state == .on
        }
        dict["excluded_apps"] = excludedField.stringValue
            .split(whereSeparator: { $0 == "," || $0 == "\n" })
            .map { String($0).trimmingCharacters(in: .whitespaces).lowercased() }
            .filter { !$0.isEmpty }
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

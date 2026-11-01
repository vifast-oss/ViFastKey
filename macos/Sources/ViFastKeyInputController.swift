import Cocoa
import InputMethodKit
import Carbon

private let MAX_RAW = 24

/// IMK input controller thật: nhận NSEvent, gọi engine Rust, áp Action lên text client.
@objc(ViFastKeyInputController)
final class ViFastKeyInputController: IMKInputController {
    private var engine: VfkEngine?
    private let applier = ActionApplier()
    private var currentAppID: String?
    private var currentClient: AnyObject?
    private var configTimer: Timer?
    private var lastConfigMtime: Date?
    /// Mặc định toàn cục: 0 = PREEDIT, 1 = REPLACE (không gạch chân).
    private var globalOutputMode: UInt32 = 1

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        ConfigFile.ensureExists()
        currentClient = inputClient as AnyObject?
        currentAppID = NSWorkspace.shared.frontmostApplication?.bundleIdentifier
        globalOutputMode = Self.modeValue(ConfigFile.outputMode)
        engine = VfkEngine(configJSON: ConfigFile.load(), mode: globalOutputMode)
        if let app = currentAppID {
            _ = engine?.focusChanged(appID: app)
        }
        NotificationCenter.default.addObserver(forName: .viFastKeyConfigChanged,
                                               object: nil, queue: .main) { [weak self] _ in
            self?.reloadConfig(nil)
        }
        startConfigWatcher()
    }

    deinit {
        configTimer?.invalidate()
    }

    /// UI cấu hình chạy ở process khác, nên NotificationCenter không đủ.
    /// Poll mtime 1 giây; Fcitx5 cũng dùng cùng pattern.
    private func startConfigWatcher() {
        lastConfigMtime = ConfigFile.modificationDate()
        let timer = Timer(timeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.pollConfigFile()
        }
        RunLoop.main.add(timer, forMode: .common)
        configTimer = timer
    }

    private func pollConfigFile() {
        let date = ConfigFile.modificationDate()
        if date == lastConfigMtime {
            return
        }
        lastConfigMtime = date
        reloadConfig(nil)
    }

    // MARK: - IMK lifecycle

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        ConfigFile.ensureExists()
        currentClient = sender as AnyObject?
        currentAppID = appID(for: sender)
        if engine == nil {
            engine = VfkEngine(configJSON: ConfigFile.load())
        }
        _ = engine?.focusChanged(appID: currentAppID)
        // Preedit cũ không được tồn tại khi vừa chuyển context.
        _ = engine?.cursorMoved()
        applyOutputMode(for: sender)
    }

    override func deactivateServer(_ sender: Any!) {
        commitComposition(sender)
        super.deactivateServer(sender)
        currentClient = nil
    }

    override func commitComposition(_ sender: Any!) {
        guard let engine else { return }
        let action = engine.cursorMoved()
        if action.kind != .pass, let client = clientObject(sender) {
            _ = applier.apply(action, to: ImkTextClient(client))
        }
    }

    // MARK: - Phím

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else { return false }
        guard let engine else { return false }
        currentClient = sender as AnyObject?
        currentAppID = appID(for: sender) ?? currentAppID

        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        let ctrl = flags.contains(.control)
        let shift = flags.contains(.shift)
        let alt = flags.contains(.option)
        let meta = flags.contains(.command)

        // Password/secure fields: không được đưa nội dung vào preedit/logic.
        if IsSecureEventInputEnabled() {
            // Không commit preedit vào secure field: xoá marked text và bỏ state.
            engine.reset()
            if let client = clientObject(sender) {
                ImkTextClient(client).setMarkedText(
                    "", selectedRange: NSRange(location: 0, length: 0),
                    replacementRange: NSRange(location: NSNotFound, length: 0)
                )
            }
            return false
        }

        // Alt+,: toggle Việt/Anh ngay trong app; không cho engine xử lý phím này.
        if alt && !ctrl && !shift && !meta && event.keyCode == kVK_ANSI_Comma {
            toggleVietnamese(nil)
            return true
        }

        let beforeDisplay = engine.displayText
        var snapshot: (text: String, cursorUTF16: UInt32, anchorUTF16: UInt32)?
        if let client = clientObject(sender) {
            let textClient = ImkTextClient(client)
            snapshot = textClient.surroundingSnapshot(maxLength: MAX_RAW + 1)
            engine.setSurroundingText(textClient.beforeCursor(MAX_RAW + 1))
        }

        let keyName = KeyMapper.name(for: event.keyCode)
        let action: VfkEngineAction
        if let keyName {
            action = engine.processKey(keyName, ctrl: ctrl, alt: alt, meta: meta)
        } else if !ctrl && !alt && !meta,
                  let chars = event.characters,
                  chars.unicodeScalars.count == 1,
                  let scalar = chars.unicodeScalars.first,
                  !CharacterSet.controlCharacters.contains(scalar) {
            action = engine.processKey(chars)
        } else {
            // Phím không phải text / shortcut: chốt từ rồi để app xử lý.
            let boundary = engine.processKey("", ctrl: ctrl, alt: alt, meta: meta)
            if boundary.kind != .pass, let client = clientObject(sender) {
                _ = applier.apply(boundary, to: ImkTextClient(client))
            }
            return false
        }

        guard let client = clientObject(sender) else { return false }
        let textClient = ImkTextClient(client)
        let plan = engine.planAction(
            action,
            fieldKind: 0,
            beforeDisplay: beforeDisplay,
            surroundingText: snapshot?.text,
            cursorUTF16: snapshot?.cursorUTF16 ?? 0,
            anchorUTF16: snapshot?.anchorUTF16 ?? 0
        )
        if plan.kind == .failSafe {
            engine.reset()
            textClient.setMarkedText("", selectedRange: NSRange(location: 0, length: 0),
                                     replacementRange: NSRange(location: NSNotFound, length: 0))
            return false
        }
        let result = applier.apply(plan, to: textClient)
        return result.handled && !result.forwardKey
    }

    private static func modeValue(_ name: String) -> UInt32 {
        name == "preedit" ? 0 : 1
    }

    /// App profile được ưu tiên; nếu app không có profile thì dùng mặc định
    /// toàn cục (Replace = không gạch chân).
    private func applyOutputMode(for sender: Any!) {
        guard let engine, let client = clientObject(sender) else { return }
        let target: UInt32
        switch engine.profileOutputMode {
        case 1: target = 0                      // Preedit
        case 2: target = 1                      // Replace
        default: target = globalOutputMode
        }
        let action = engine.setOutputMode(target)
        if action.kind != VfkActionKind.pass {
            _ = applier.apply(action, to: ImkTextClient(client))
        }
    }

    private func clientObject(_ sender: Any!) -> AnyObject? {
        if let c = sender as AnyObject? { return c }
        return currentClient
    }

    private func appID(for sender: Any!) -> String? {
        // IMK client thường không lộ bundleIdentifier qua protocol công khai.
        // Frontmost app tại thời điểm activate chính là app đích.
        if let app = NSWorkspace.shared.frontmostApplication?.bundleIdentifier,
           app != ConfigFile.bundleID {
            return app
        }
        return currentAppID
    }

    // MARK: - Menu

    override func menu() -> NSMenu! {
        let menu = NSMenu(title: "ViFastKey")
        let enabled = engine?.isEnabled ?? true
        let toggle = NSMenuItem(title: enabled ? "Tắt tiếng Việt" : "Bật tiếng Việt",
                                action: #selector(toggleVietnamese(_:)), keyEquivalent: ",")
        toggle.keyEquivalentModifierMask = [.option]
        toggle.target = self
        menu.addItem(toggle)

        menu.addItem(.separator())
        let methodTitle = engine?.configJSON.flatMap(methodName) ?? "Telex"
        let methodItem = NSMenuItem(title: "Kiểu gõ: \(methodTitle)", action: nil, keyEquivalent: "")
        let submenu = NSMenu(title: "Kiểu gõ")
        for (name, label) in [("telex", "Telex"), ("simple_telex", "Simple Telex"), ("vni", "VNI"), ("viqr", "VIQR")] {
            let item = NSMenuItem(title: label, action: #selector(selectMethod(_:)), keyEquivalent: "")
            item.target = self
            item.representedObject = name
            item.state = (name == currentMethod) ? .on : .off
            submenu.addItem(item)
        }
        methodItem.submenu = submenu
        menu.addItem(methodItem)

        menu.addItem(.separator())
        let prefs = NSMenuItem(title: "Cấu hình…", action: #selector(showPreferences(_:)),
                               keyEquivalent: ",")
        prefs.target = self
        menu.addItem(prefs)
        let openJSON = NSMenuItem(title: "Mở file JSON", action: #selector(openConfigFile(_:)),
                                  keyEquivalent: "")
        openJSON.target = self
        menu.addItem(openJSON)
        let reload = NSMenuItem(title: "Nạp lại cấu hình", action: #selector(reloadConfig(_:)),
                                keyEquivalent: "")
        reload.target = self
        menu.addItem(reload)

        let sources = NSMenuItem(title: "Mở Input Sources…",
                                 action: #selector(openInputSources(_:)), keyEquivalent: "")
        sources.target = self
        menu.addItem(sources)

        let clipboard = NSMenuItem(title: "Clipboard", action: nil, keyEquivalent: "")
        let clipboardMenu = NSMenu(title: "Clipboard")
        addClipboardItem(clipboardMenu, "Bỏ dấu", op: "strip")
        addClipboardItem(clipboardMenu, "Chuẩn hoá Unicode NFC", op: "nfc")
        addClipboardItem(clipboardMenu, "VIQR → Unicode", op: "decode", charset: "viqr")
        addClipboardItem(clipboardMenu, "Unicode → VIQR", op: "encode", charset: "viqr")
        clipboardMenu.addItem(.separator())
        addClipboardItem(clipboardMenu, "TCVN3 → Unicode", op: "decode", charset: "tcvn3")
        addClipboardItem(clipboardMenu, "Unicode → TCVN3 (UTF-8 thay thế)", op: "encode", charset: "tcvn3")
        clipboard.submenu = clipboardMenu
        menu.addItem(clipboard)
        return menu
    }

    private var currentMethod: String {
        guard let json = engine?.configJSON,
              let data = json.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let method = obj["input_method"] as? String else { return "telex" }
        return method
    }

    private func methodName(_ json: String) -> String {
        guard let data = json.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let method = obj["input_method"] as? String else { return "Telex" }
        switch method {
        case "vni": return "VNI"
        case "simple_telex": return "Simple Telex"
        case "viqr": return "VIQR"
        default: return "Telex"
        }
    }

    @objc private func toggleVietnamese(_ sender: Any?) {
        guard let engine else { return }
        let action = engine.toggleVietnamese()
        if action.kind != .pass, let client = currentClient {
            _ = applier.apply(action, to: ImkTextClient(client))
        }
        persistEngineConfig()
    }

    @objc private func selectMethod(_ sender: NSMenuItem) {
        guard let method = sender.representedObject as? String else { return }
        if let client = currentClient, let action = engine?.cursorMoved(), action.kind != .pass {
            _ = applier.apply(action, to: ImkTextClient(client))
        }
        ConfigFile.update { $0["input_method"] = method }
        engine?.setConfigJSON("{\"input_method\":\"\(method)\"}")
        persistEngineConfig()
    }

    override func showPreferences(_ sender: Any?) {
        PreferencesWindowController.shared.show()
    }

    @objc private func openConfigFile(_ sender: Any?) {
        ConfigFile.ensureExists()
        NSWorkspace.shared.open(ConfigFile.path)
    }

    @objc private func openInputSources(_ sender: Any?) {
        NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.Keyboard-Settings.extension")!)
    }

    private func addClipboardItem(_ menu: NSMenu, _ title: String, op: String, charset: String? = nil) {
        let item = NSMenuItem(title: title, action: #selector(transformClipboard(_:)), keyEquivalent: "")
        item.target = self
        item.representedObject = ["op": op, "charset": charset ?? ""]
        menu.addItem(item)
    }

    @objc private func transformClipboard(_ sender: NSMenuItem) {
        guard let spec = sender.representedObject as? [String: String],
              let op = spec["op"],
              let text = NSPasteboard.general.string(forType: .string) else {
            NSSound.beep()
            return
        }
        let charset = spec["charset"] ?? ""
        let output: String?
        switch op {
        case "strip":
            output = text.applyingTransform(.stripDiacritics, reverse: false)
        case "nfc":
            output = text.precomposedStringWithCanonicalMapping
        case "decode":
            output = engine?.decodeCharset(text, charset: charset)
        case "encode":
            output = engine?.encodeCharset(text, charset: charset)
        default:
            output = nil
        }
        guard let output else {
            NSSound.beep()
            return
        }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(output, forType: .string)
    }

    @objc private func reloadConfig(_ sender: Any?) {
        if let client = currentClient, let action = engine?.cursorMoved(), action.kind != .pass {
            _ = applier.apply(action, to: ImkTextClient(client))
        }
        globalOutputMode = Self.modeValue(ConfigFile.outputMode)
        engine = VfkEngine(configJSON: ConfigFile.load(), mode: globalOutputMode)
        _ = engine?.focusChanged(appID: currentAppID)
        applyOutputMode(for: currentClient)
        lastConfigMtime = ConfigFile.modificationDate()
    }

    /// Chỉ ghi runtime state do adapter sở hữu (`app_modes`), không ghi toàn bộ
    /// engine config: bản đầy đủ sẽ materialize preset và xoá key lạ do UI/bản mới ghi.
    private func persistEngineConfig() {
        guard let json = engine?.configJSON,
              let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let appModes = object["app_modes"] else { return }
        ConfigFile.update { $0["app_modes"] = appModes }
        lastConfigMtime = ConfigFile.modificationDate()
    }
}

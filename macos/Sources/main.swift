import Cocoa
import InputMethodKit
import Carbon

ConfigFile.ensureExists()

private let inputSourcePrefix = "app.vifastkey.inputmethod"

func strProperty(_ src: TISInputSource, _ key: CFString) -> String? {
    guard let valuep = TISGetInputSourceProperty(src, key) else { return nil }
    return Unmanaged<CFString>.fromOpaque(valuep).takeUnretainedValue() as String
}

func boolProperty(_ src: TISInputSource, _ key: CFString) -> Bool {
    guard let valuep = TISGetInputSourceProperty(src, key) else { return false }
    return CFBooleanGetValue(Unmanaged<CFBoolean>.fromOpaque(valuep).takeUnretainedValue())
}

/// Các input source ViFastKey đã đăng ký với HIToolbox.
func viFastKeySources() -> [TISInputSource] {
    let all = TISCreateInputSourceList(nil, true)?.takeRetainedValue() as? [TISInputSource] ?? []
    return all.filter { src in
        (strProperty(src, kTISPropertyInputSourceID) ?? "").hasPrefix(inputSourcePrefix)
    }
}

func sourceID(_ src: TISInputSource) -> String {
    strProperty(src, kTISPropertyInputSourceID) ?? "?"
}

func printStatus() {
    let sources = viFastKeySources()
    if sources.isEmpty {
        print("ViFastKey: chưa đăng ký input source")
    }
    for src in sources {
        let enabled = boolProperty(src, kTISPropertyInputSourceIsEnabled)
        let selected = boolProperty(src, kTISPropertyInputSourceIsSelected)
        print("\(sourceID(src)): enabled=\(enabled) selected=\(selected)")
    }
    print("prefs AppleEnabledInputSources: \(InputSourceRegistrar.isEnabled())")
    print("output_mode: \(ConfigFile.outputMode)")
}

func maintenance() -> Bool {
    let args = CommandLine.arguments
    if args.contains("--preferences") {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        PreferencesWindowController.shared.show()
        app.run()
        return true
    }

    let wantsRegister = args.contains("--register-input-source")
    let wantsEnable = args.contains("--enable-input-source")
    let wantsRepair = args.contains("--repair-input-source")
    let wantsSelect = args.contains("--select-input-source")
    let wantsStatus = args.contains("--status")
    guard wantsRegister || wantsEnable || wantsRepair || wantsSelect || wantsStatus else {
        return false
    }

    if wantsStatus {
        printStatus()
    }
    if wantsRegister || wantsRepair {
        let status = TISRegisterInputSource(Bundle.main.bundleURL as CFURL)
        print("register status: \(status)")
    }
    if wantsEnable || wantsRepair {
        for src in viFastKeySources()
        where !boolProperty(src, kTISPropertyInputSourceIsEnabled) {
            print("enable \(sourceID(src)): \(TISEnableInputSource(src))")
        }
    }
    if wantsRepair {
        // Fallback cũ cho máy không lưu trạng thái enable sau logout. Chỉ chạy
        // khi người dùng chủ động sửa, vì ghi thẳng prefs có thể tạo entry
        // "enabled" trong khi TIS vẫn coi là chưa bật.
        print("prefs fallback: \(InputSourceRegistrar.ensureEnabled())")
    }
    if wantsSelect {
        for src in viFastKeySources() {
            print("select \(sourceID(src)): \(TISSelectInputSource(src))")
        }
    }
    return true
}

if maintenance() {
    exit(0)
}

// Không đăng ký/bật input source ở đây.
//
// IMK chỉ khởi động app khi input source đã được người dùng bật trong
// System Settings. Gọi TISRegisterInputSource/TISEnableInputSource ở mỗi lần
// chạy có thể khiến macOS coi như vừa thêm input source mới và hỏi lại
// "Allow ViFastKey to enable ViFastKey?" sau mỗi lần khởi động/chuyển app.
// Đăng ký là việc của installer (--register-input-source).

let connectionName = "ViFastKey_Connection"
let bundleIdentifier = Bundle.main.bundleIdentifier ?? "app.vifastkey.inputmethod"
let server = IMKServer(name: connectionName, bundleIdentifier: bundleIdentifier)
_ = server
NSApplication.shared.run()

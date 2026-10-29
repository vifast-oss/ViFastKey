import Cocoa
import InputMethodKit
import Carbon

ConfigFile.ensureExists()

func maintenance() -> Bool {
    let args = CommandLine.arguments
    if args.contains("--preferences") {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        PreferencesWindowController.shared.show()
        app.run()
        return true
    }
    guard args.contains("--register-input-source") ||
          args.contains("--enable-input-source") ||
          args.contains("--repair-input-source") else {
        return false
    }
    let bundleURL = Bundle.main.bundleURL
    let status = TISRegisterInputSource(bundleURL as CFURL)
    print("register status: \(status)")
    print("prefs enabled: \(InputSourceRegistrar.ensureEnabled())")
    let all = TISCreateInputSourceList(nil, true)?.takeRetainedValue() as? [TISInputSource] ?? []
    for src in all {
        guard let idp = TISGetInputSourceProperty(src, kTISPropertyInputSourceID) else { continue }
        let id = Unmanaged<CFString>.fromOpaque(idp).takeUnretainedValue() as String
        guard id.contains("app.vifastkey.inputmethod") else { continue }
        print("enable \(id): \(TISEnableInputSource(src))")
        if args.contains("--select-input-source") {
            print("select \(id): \(TISSelectInputSource(src))")
        }
    }
    return true
}

if maintenance() {
    exit(0)
}

// Tự bật input source khi IMK khởi động app (nếu chưa được bật).
do {
    let all = TISCreateInputSourceList(nil, true)?.takeRetainedValue() as? [TISInputSource] ?? []
    for src in all {
        guard let idp = TISGetInputSourceProperty(src, kTISPropertyInputSourceID) else { continue }
        let id = Unmanaged<CFString>.fromOpaque(idp).takeUnretainedValue() as String
        if id.contains("app.vifastkey.inputmethod") {
            _ = TISEnableInputSource(src)
        }
    }
}

let connectionName = "ViFastKey_Connection"
let bundleIdentifier = Bundle.main.bundleIdentifier ?? "app.vifastkey.inputmethod"
let server = IMKServer(name: connectionName, bundleIdentifier: bundleIdentifier)
_ = server
NSApplication.shared.run()

import Foundation

/// Đảm bảo ViFastKey nằm trong AppleEnabledInputSources của HIToolbox.
///
/// `TISEnableInputSource` trả success nhưng trên một số bản macOS không ghi
/// preference bền; hàm này ghi bổ sung để input source còn sau logout/login.
enum InputSourceRegistrar {
    private static let domain = "com.apple.HIToolbox" as CFString
    private static let key = "AppleEnabledInputSources" as CFString

    static func ensureEnabled() -> Bool {
        let bundleID = ConfigFile.bundleID
        var sources = (CFPreferencesCopyAppValue(key, domain) as? [[String: Any]]) ?? []
        let exists = sources.contains { item in
            (item["Bundle ID"] as? String) == bundleID
        }
        if !exists {
            sources.append([
                "Bundle ID": bundleID,
                "InputSourceKind": "Keyboard Input Method",
            ])
            CFPreferencesSetAppValue(key, sources as CFArray, domain)
            _ = CFPreferencesAppSynchronize(domain)
        }
        return true
    }

    static func isEnabled() -> Bool {
        let bundleID = ConfigFile.bundleID
        let sources = (CFPreferencesCopyAppValue(key, domain) as? [[String: Any]]) ?? []
        return sources.contains { ($0["Bundle ID"] as? String) == bundleID }
    }
}

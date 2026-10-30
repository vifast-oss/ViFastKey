import Foundation

struct ConfigFile {
    static let bundleID = "app.vifastkey.inputmethod.ViFastKey"

    static var directory: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("ViFastKey", isDirectory: true)
    }

    static var path: URL {
        directory.appendingPathComponent("config.json")
    }

    static func load() -> String? {
        try? String(contentsOf: path, encoding: .utf8)
    }

    static func modificationDate() -> Date? {
        (try? FileManager.default.attributesOfItem(atPath: path.path))?[.modificationDate] as? Date
    }

    static func loadDictionary() -> [String: Any] {
        guard let data = try? Data(contentsOf: path),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return [:]
        }
        return obj
    }

    static func saveDictionary(_ dict: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: dict,
                                                     options: [.prettyPrinted, .sortedKeys]),
              let text = String(data: data, encoding: .utf8) else { return }
        save(text)
    }

    static func ensureExists() {
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        if !FileManager.default.fileExists(atPath: path.path) {
            let sample = """
            {
              "schema_version": 1,
              "input_method": "telex",
              "tone_style": "new",
              "spell_check": true,
              "w_as_u": false,
              "bracket_keys": false,
              "quick_telex": false,
              "allow_coda_k": false,
              "auto_restore_english": true,
              "macro_enabled": true,
              "macro_digits": false,
              "macro_undo": true,
              "macros": {"ko": "không", "vn": "Việt Nam"},
              "auto_capitalize": false,
              "excluded_apps": ["minecraft"],
              "default_vietnamese": true,
              "edit_previous_word": true,
              "restore_after_space": true,
              "smart_caps": false,
              "unicode_form": "NFC"
            }
            """
            try? sample.write(to: path, atomically: true, encoding: .utf8)
        }
    }

    /// Ghi atomic; giữ bản backup .bak trước khi thay.
    static func save(_ json: String) {
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        if FileManager.default.fileExists(atPath: path.path) {
            let backup = path.appendingPathExtension("bak")
            try? FileManager.default.removeItem(at: backup)
            try? FileManager.default.copyItem(at: path, to: backup)
        }
        try? json.write(to: path, atomically: true, encoding: .utf8)
    }

    static func update(_ mutate: (inout [String: Any]) -> Void) {
        var dict: [String: Any] = [:]
        if let data = try? Data(contentsOf: path),
           let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            dict = obj
        }
        mutate(&dict)
        if let data = try? JSONSerialization.data(withJSONObject: dict, options: [.prettyPrinted, .sortedKeys]),
           let text = String(data: data, encoding: .utf8) {
            save(text)
        }
    }
}

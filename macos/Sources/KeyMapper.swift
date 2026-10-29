import Foundation

/// Ánh xạ keyCode macOS -> tên phím của engine. Tách khỏi IMK để test được.
enum KeyMapper {
    static func name(for keyCode: UInt16) -> String? {
        switch keyCode {
        case 51: return "Backspace"
        case 36, 76: return "Enter"
        case 48: return "Tab"
        case 53: return "Escape"
        case 123: return "Left"
        case 124: return "Right"
        case 125: return "Down"
        case 126: return "Up"
        case 115: return "Home"
        case 119: return "End"
        case 117: return "Delete"
        case 116: return "PageUp"
        case 121: return "PageDown"
        default: return nil
        }
    }
}

#include "keymap.h"

#include "utf.h"

namespace {

bool IsLetterOrDigit(WPARAM vk) {
    return (vk >= 'A' && vk <= 'Z') || (vk >= '0' && vk <= '9');
}

bool IsOemKey(WPARAM vk) {
    return (vk >= VK_OEM_1 && vk <= VK_OEM_3) || (vk >= VK_OEM_4 && vk <= VK_OEM_102) ||
           (vk >= VK_NUMPAD0 && vk <= VK_DIVIDE);
}

}  // namespace

bool IsToggleKey(WPARAM vk, bool ctrl, bool shift) {
    return ctrl && shift && vk == 'F';
}

bool IsEngineKey(WPARAM vk) {
    switch (vk) {
        case VK_BACK:
        case VK_RETURN:
        case VK_ESCAPE:
        case VK_TAB:
        case VK_LEFT:
        case VK_RIGHT:
        case VK_UP:
        case VK_DOWN:
        case VK_HOME:
        case VK_END:
        case VK_DELETE:
        case VK_PRIOR:
        case VK_NEXT:
        case VK_SPACE:
            return true;
        default:
            break;
    }
    return IsLetterOrDigit(vk) || IsOemKey(vk);
}

std::string KeyFromVk(WPARAM vk, LPARAM lParam) {
    switch (vk) {
        case VK_BACK:
            return "Backspace";
        case VK_RETURN:
            return "Enter";
        case VK_ESCAPE:
            return "Escape";
        case VK_TAB:
            return "Tab";
        case VK_LEFT:
            return "Left";
        case VK_RIGHT:
            return "Right";
        case VK_UP:
            return "Up";
        case VK_DOWN:
            return "Down";
        case VK_HOME:
            return "Home";
        case VK_END:
            return "End";
        case VK_DELETE:
            return "Delete";
        case VK_PRIOR:
            return "PageUp";
        case VK_NEXT:
            return "PageDown";
        case VK_SPACE:
            return " ";
        case VK_CONTROL:
        case VK_LCONTROL:
        case VK_RCONTROL:
            return "Control";
        case VK_MENU:
        case VK_LMENU:
        case VK_RMENU:
            return "Alt";
        case VK_SHIFT:
        case VK_LSHIFT:
        case VK_RSHIFT:
            return "Shift";
        case VK_LWIN:
        case VK_RWIN:
            return "Meta";
        case VK_CAPITAL:
            return "CapsLock";
        default:
            break;
    }

    // Layout có thể là US, UK, AZERTY... ToUnicodeEx cho đúng ký tự mà app sẽ nhận.
    BYTE state[256] = {};
    if (GetKeyboardState(state) == FALSE) {
        return {};
    }
    const UINT scan = static_cast<UINT>((lParam >> 16) & 0xFF);
    const HKL layout = GetKeyboardLayout(GetCurrentThreadId());
    wchar_t buffer[8] = {};
    int count = ToUnicodeEx(static_cast<UINT>(vk), scan, state, buffer, 8, 0, layout);
    if (count <= 0) {
        return {};
    }
    return WideToUtf8(std::wstring(buffer, static_cast<size_t>(count)));
}

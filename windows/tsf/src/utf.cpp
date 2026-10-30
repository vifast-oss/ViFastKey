#include "utf.h"

#include <windows.h>

std::wstring Utf8ToWide(const std::string& text) {
    if (text.empty()) {
        return {};
    }
    int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                                    static_cast<int>(text.size()), nullptr, 0);
    if (count <= 0) {
        return {};
    }
    std::wstring out(static_cast<size_t>(count), L'\0');
    MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                        static_cast<int>(text.size()), out.data(), count);
    return out;
}

std::string WideToUtf8(const std::wstring& text) {
    if (text.empty()) {
        return {};
    }
    int count = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                                    static_cast<int>(text.size()), nullptr, 0, nullptr, nullptr);
    if (count <= 0) {
        return {};
    }
    std::string out(static_cast<size_t>(count), '\0');
    WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                        static_cast<int>(text.size()), out.data(), count, nullptr, nullptr);
    return out;
}

uint32_t Utf16Len(const std::wstring& text) {
    return static_cast<uint32_t>(text.size());
}

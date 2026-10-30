#include "config.h"

#include <windows.h>

#include <cctype>
#include <filesystem>
#include <fstream>
#include <iterator>

namespace {

std::wstring EnvironmentValue(const wchar_t* name) {
    DWORD size = GetEnvironmentVariableW(name, nullptr, 0);
    if (size == 0) {
        return {};
    }
    std::wstring value(size, L'\0');
    DWORD written = GetEnvironmentVariableW(name, value.data(), size);
    if (written == 0 || written >= size) {
        return {};
    }
    value.resize(written);
    return value;
}

std::string ReadFile(const std::filesystem::path& path) {
    std::ifstream file(path, std::ios::binary);
    if (!file) {
        return {};
    }
    return std::string(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
}

// windows.json chỉ có vài trường; tránh kéo thêm thư viện JSON vào DLL TSF.
std::string ParseBackend(const std::string& json) {
    const std::string key = "\"backend\"";
    size_t pos = json.find(key);
    if (pos == std::string::npos) {
        return "tsf";
    }
    pos = json.find(':', pos + key.size());
    if (pos == std::string::npos) {
        return "tsf";
    }
    ++pos;
    while (pos < json.size() && std::isspace(static_cast<unsigned char>(json[pos]))) {
        ++pos;
    }
    if (pos >= json.size() || json[pos] != '"') {
        return "tsf";
    }
    ++pos;
    size_t end = json.find('"', pos);
    if (end == std::string::npos) {
        return "tsf";
    }
    std::string value = json.substr(pos, end - pos);
    for (char& ch : value) {
        ch = static_cast<char>(std::tolower(static_cast<unsigned char>(ch)));
    }
    if (value == "hook" || value == "tsf" || value == "auto") {
        return value;
    }
    return "tsf";
}

}  // namespace

std::wstring ConfigDirectory() {
    std::wstring appdata = EnvironmentValue(L"APPDATA");
    if (appdata.empty()) {
        return L".";
    }
    return (std::filesystem::path(appdata) / L"ViFastKey").wstring();
}

AppConfig LoadAppConfig() {
    AppConfig config;
    const std::filesystem::path dir(ConfigDirectory());
    config.engine_json = ReadFile(dir / L"config.json");
    config.backend = ParseBackend(ReadFile(dir / L"windows.json"));
    return config;
}

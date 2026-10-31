#pragma once

#include <string>

struct AppConfig {
    // Nội dung %APPDATA%\ViFastKey\config.json (JSON đầy đủ hoặc một phần).
    std::string engine_json;
    // hook | tsf. Mặc định tsf (preedit); auto là giá trị legacy.
    std::string backend = "tsf";
};

AppConfig LoadAppConfig();

// Thư mục %APPDATA%\ViFastKey (không tự tạo).
std::wstring ConfigDirectory();

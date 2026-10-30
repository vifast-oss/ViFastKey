#pragma once

#include <string>

struct AppConfig {
    // Nội dung %APPDATA%\ViFastKey\config.json (JSON đầy đủ hoặc một phần).
    std::string engine_json;
    // hook | tsf | auto. Mặc định tsf (preedit).
    std::string backend = "tsf";
};

AppConfig LoadAppConfig();

// Thư mục %APPDATA%\ViFastKey (không tự tạo).
std::wstring ConfigDirectory();

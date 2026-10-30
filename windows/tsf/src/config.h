#pragma once

#include <string>

struct AppConfig {
    // Nội dung %APPDATA%\ViFastKey\config.json (JSON đầy đủ hoặc một phần).
    std::string engine_json;
    // hook | tsf | auto. Mặc định auto.
    std::string backend = "auto";
};

AppConfig LoadAppConfig();

// Thư mục %APPDATA%\ViFastKey (không tự tạo).
std::wstring ConfigDirectory();

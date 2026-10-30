#pragma once

#include <windows.h>

#include <string>

// Phím đặc biệt hoặc ký tự UTF-8 tương ứng với VK. Trả chuỗi rỗng nếu không
// chuyển được (dead key, tổ hợp lạ, phím điều khiển...).
std::string KeyFromVk(WPARAM vk, LPARAM lParam);

// Có phải phím mà engine có thể xử lý (kể cả phím đặc biệt)?
bool IsEngineKey(WPARAM vk);

// Ctrl+Shift+F: toggle Việt/Anh.
bool IsToggleKey(WPARAM vk, bool ctrl, bool shift);

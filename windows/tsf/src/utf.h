#pragma once

#include <cstdint>
#include <string>

std::wstring Utf8ToWide(const std::string& text);
std::string WideToUtf8(const std::wstring& text);
uint32_t Utf16Len(const std::wstring& text);

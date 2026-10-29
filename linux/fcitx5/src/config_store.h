#pragma once

#include <chrono>
#include <cstdint>
#include <filesystem>
#include <string>

namespace fcitx {

// Đọc config JSON của ViFastKey và theo dõi thay đổi file.
//
// Adapter Fcitx5 không dùng cơ chế config của Fcitx mà dùng chung
// ~/.config/vifastkey/config.json với macOS/IBus. File được kiểm tra mtime
// theo chu kỳ ngắn; nội dung chỉ được đọc lại khi mtime đổi.
class ConfigStore {
public:
    explicit ConfigStore(std::string path = {});

    const std::string &json() const { return json_; }
    uint64_t generation() const { return generation_; }
    const std::string &path() const { return path_; }

    // force=false: bỏ qua nếu chưa đủ chu kỳ kiểm tra.
    void refresh(bool force = false);

private:
    static std::string defaultPath();
    static bool readFile(const std::string &path, std::string &out);

    std::string path_;
    std::string json_;
    uint64_t generation_ = 0;
    std::filesystem::file_time_type mtime_{};
    std::chrono::steady_clock::time_point lastCheck_{};
    bool lastReadFailed_ = false;
};

} // namespace fcitx

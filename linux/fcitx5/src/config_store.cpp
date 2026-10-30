#include "config_store.h"

#include <fcitx-utils/log.h>

#include <cstdlib>
#include <fstream>
#include <iterator>
#include <system_error>
#include <utility>

namespace fcitx {
namespace {

constexpr auto kRefreshInterval = std::chrono::milliseconds(500);

} // namespace

ConfigStore::ConfigStore(std::string path)
    : path_(path.empty() ? defaultPath() : std::move(path)),
      userWordsPath_(defaultUserWordsPath(path_)) {
    refresh(true);
}

std::string ConfigStore::defaultPath() {
    if (const char *overridePath = std::getenv("VIFASTKEY_CONFIG");
        overridePath && *overridePath) {
        return overridePath;
    }
    if (const char *xdg = std::getenv("XDG_CONFIG_HOME"); xdg && *xdg) {
        return std::string(xdg) + "/vifastkey/config.json";
    }
    if (const char *home = std::getenv("HOME"); home && *home) {
        return std::string(home) + "/.config/vifastkey/config.json";
    }
    return {};
}

std::string ConfigStore::defaultUserWordsPath(const std::string &configPath) {
    if (const char *overridePath = std::getenv("VIFASTKEY_USER_WORDS");
        overridePath && *overridePath) {
        return overridePath;
    }
    if (configPath.empty()) {
        return {};
    }
    return (std::filesystem::path(configPath).parent_path() / "user_words.txt")
        .string();
}

bool ConfigStore::readFile(const std::string &path, std::string &out) {
    if (path.empty()) {
        return false;
    }
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        return false;
    }
    out.assign(std::istreambuf_iterator<char>(in),
               std::istreambuf_iterator<char>());
    return true;
}

void ConfigStore::refreshUserWords(bool force) {
    if (userWordsPath_.empty()) {
        return;
    }
    const auto now = std::chrono::steady_clock::now();
    if (!force && now - lastUserWordsCheck_ < kRefreshInterval) {
        return;
    }
    lastUserWordsCheck_ = now;

    std::error_code ec;
    const auto mtime =
        std::filesystem::last_write_time(std::filesystem::path(userWordsPath_), ec);
    if (ec) {
        if (ec == std::errc::no_such_file_or_directory) {
            if (!userWords_.empty()) {
                userWords_.clear();
                ++userWordsGeneration_;
                FCITX_INFO() << "vifastkey: user_words removed";
            }
            lastUserWordsReadFailed_ = false;
        } else if (!lastUserWordsReadFailed_) {
            lastUserWordsReadFailed_ = true;
            FCITX_WARN() << "vifastkey: cannot stat user_words path="
                         << userWordsPath_ << " error=" << ec.message();
        }
        return;
    }
    if (!force && mtime == userWordsMtime_) {
        return;
    }

    std::string content;
    if (!readFile(userWordsPath_, content)) {
        if (!lastUserWordsReadFailed_) {
            lastUserWordsReadFailed_ = true;
            FCITX_WARN() << "vifastkey: cannot read user_words path="
                         << userWordsPath_;
        }
        return;
    }
    lastUserWordsReadFailed_ = false;
    userWordsMtime_ = mtime;
    if (content == userWords_) {
        return;
    }
    userWords_ = std::move(content);
    ++userWordsGeneration_;
    FCITX_INFO() << "vifastkey: user_words loaded path=" << userWordsPath_
                 << " bytes=" << userWords_.size()
                 << " generation=" << userWordsGeneration_;
}

void ConfigStore::refresh(bool force) {
    const auto now = std::chrono::steady_clock::now();
    refreshUserWords(force);
    if (!force && now - lastCheck_ < kRefreshInterval) {
        return;
    }
    lastCheck_ = now;

    if (path_.empty()) {
        return;
    }

    std::error_code ec;
    const auto mtime =
        std::filesystem::last_write_time(std::filesystem::path(path_), ec);
    if (ec) {
        // File chưa tồn tại là trạng thái hợp lệ: dùng config mặc định.
        if (ec == std::errc::no_such_file_or_directory) {
            if (!json_.empty()) {
                json_.clear();
                ++generation_;
                FCITX_INFO() << "vifastkey: config removed, using defaults";
            }
            lastReadFailed_ = false;
        } else if (!lastReadFailed_) {
            lastReadFailed_ = true;
            FCITX_WARN() << "vifastkey: cannot stat config path=" << path_
                         << " error=" << ec.message();
        }
        return;
    }

    if (!force && mtime == mtime_) {
        return;
    }

    std::string content;
    if (!readFile(path_, content)) {
        if (!lastReadFailed_) {
            lastReadFailed_ = true;
            FCITX_WARN() << "vifastkey: cannot read config path=" << path_;
        }
        return;
    }

    lastReadFailed_ = false;
    mtime_ = mtime;
    if (content == json_) {
        return;
    }

    json_ = std::move(content);
    ++generation_;
    FCITX_INFO() << "vifastkey: config reloaded path=" << path_
                 << " bytes=" << json_.size()
                 << " generation=" << generation_;
}

} // namespace fcitx

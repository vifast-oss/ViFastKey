#pragma once

#include <cstdint>
#include <string>

#include "vifastkey.h"

struct ActionData {
    uint32_t kind = VFK_PASS;
    std::string text;
    std::string before_display;
    uint32_t delete_utf16 = 0;
    uint32_t cursor_utf16 = 0;
    bool pass_key = false;
};

struct PlanData {
    uint32_t kind = VFK_PLAN_PASS;
    std::string text;
    uint32_t delete_utf16 = 0;
    uint32_t cursor_utf16 = 0;
    bool pass_key = false;
};

// Bọc C ABI vfk-ffi. Mỗi TextService có một session PREEDIT riêng.
class Engine {
  public:
    Engine() = default;
    ~Engine();

    Engine(const Engine&) = delete;
    Engine& operator=(const Engine&) = delete;

    bool Start(const std::string& config_json);
    void Stop();
    bool Reload(const std::string& config_json, ActionData* action);
    void SetSurrounding(const std::string& before_cursor);

    ActionData ProcessKey(const std::string& key, bool ctrl, bool alt, bool meta);
    ActionData FocusChanged(const std::string& app_id);
    ActionData SetField(uint32_t field_kind);
    ActionData Flush();
    ActionData CursorMoved();
    ActionData Toggle();

    PlanData Plan(const ActionData& action, const std::string& surrounding, uint32_t cursor_utf16,
                  uint32_t anchor_utf16, bool allow_unverified) const;

    std::string Display() const;
    bool Enabled() const;
    uint32_t field_kind() const { return field_kind_; }
    void Reset();

  private:
    VfkSession* session_ = nullptr;
    uint32_t field_kind_ = VFK_FIELD_TEXT;
};

ActionData CopyAction(const VfkAction& action);

#include "engine.h"

ActionData CopyAction(const VfkAction& action) {
    ActionData out;
    out.kind = action.kind;
    out.delete_utf16 = action.delete_utf16;
    out.cursor_utf16 = action.cursor_utf16;
    out.pass_key = action.pass_key;
    if (action.text != nullptr) {
        out.text = action.text;
    }
    return out;
}

Engine::~Engine() {
    Stop();
}

bool Engine::Start(const std::string& config_json) {
    Stop();
    field_kind_ = VFK_FIELD_TEXT;
    session_ = vfk_session_new(config_json.empty() ? nullptr : config_json.c_str(), VFK_MODE_PREEDIT);
    if (session_ == nullptr && !config_json.empty()) {
        // Config hỏng: vẫn chạy được với mặc định thay vì mất bộ gõ.
        session_ = vfk_session_new(nullptr, VFK_MODE_PREEDIT);
    }
    return session_ != nullptr;
}

void Engine::Stop() {
    if (session_ != nullptr) {
        vfk_session_free(session_);
        session_ = nullptr;
    }
}

void Engine::SetSurrounding(const std::string& before_cursor) {
    if (session_ == nullptr) {
        return;
    }
    vfk_set_surrounding_text(session_, before_cursor.empty() ? nullptr : before_cursor.c_str());
}

ActionData Engine::ProcessKey(const std::string& key, bool ctrl, bool alt, bool meta) {
    if (session_ == nullptr) {
        return {};
    }
    const std::string before = Display();
    ActionData out = CopyAction(vfk_process_key(session_, key.c_str(), ctrl, alt, meta));
    out.before_display = before;
    return out;
}

ActionData Engine::FocusChanged(const std::string& app_id) {
    if (session_ == nullptr) {
        return {};
    }
    const std::string before = Display();
    ActionData out =
        CopyAction(vfk_focus_changed(session_, app_id.empty() ? nullptr : app_id.c_str()));
    out.before_display = before;
    return out;
}

ActionData Engine::SetField(uint32_t field_kind) {
    if (session_ == nullptr) {
        return {};
    }
    const std::string before = Display();
    field_kind_ = field_kind;
    ActionData out = CopyAction(vfk_set_field_kind(session_, field_kind));
    out.before_display = before;
    return out;
}

ActionData Engine::Flush() {
    if (session_ == nullptr) {
        return {};
    }
    const std::string before = Display();
    ActionData out = CopyAction(vfk_flush(session_));
    out.before_display = before;
    return out;
}

ActionData Engine::CursorMoved() {
    if (session_ == nullptr) {
        return {};
    }
    const std::string before = Display();
    ActionData out = CopyAction(vfk_cursor_moved(session_));
    out.before_display = before;
    return out;
}

ActionData Engine::Toggle() {
    if (session_ == nullptr) {
        return {};
    }
    const std::string before = Display();
    ActionData out = CopyAction(vfk_toggle_vietnamese(session_));
    out.before_display = before;
    return out;
}

bool Engine::Reload(const std::string& config_json, ActionData* action) {
    if (session_ == nullptr || config_json.empty()) {
        return false;
    }
    const std::string before = Display();
    VfkAction c_action = {};
    if (!vfk_apply_config_json(session_, config_json.c_str(), &c_action)) {
        return false;
    }
    if (action != nullptr) {
        *action = CopyAction(c_action);
        action->before_display = before;
    }
    return true;
}

PlanData Engine::Plan(const ActionData& action, const std::string& surrounding,
                      uint32_t cursor_utf16, uint32_t anchor_utf16,
                      bool allow_unverified) const {
    PlanData out;
    if (session_ == nullptr) {
        out.kind = VFK_PLAN_FAIL_SAFE;
        return out;
    }

    VfkAction c_action = {};
    c_action.kind = action.kind;
    c_action.delete_utf16 = action.delete_utf16;
    c_action.cursor_utf16 = action.cursor_utf16;
    c_action.pass_key = action.pass_key;
    c_action.text = action.text.empty() ? nullptr : action.text.c_str();

    VfkPlan plan = {};
    if (!vfk_plan_action(session_, &c_action, field_kind_,
                         action.before_display.empty() ? nullptr : action.before_display.c_str(),
                         surrounding.empty() ? nullptr : surrounding.c_str(), cursor_utf16,
                         anchor_utf16, allow_unverified, &plan)) {
        out.kind = VFK_PLAN_FAIL_SAFE;
        return out;
    }

    out.kind = plan.kind;
    out.delete_utf16 = plan.delete_utf16;
    out.cursor_utf16 = plan.cursor_utf16;
    out.pass_key = plan.pass_key;
    if (plan.text != nullptr) {
        out.text = plan.text;
    }
    return out;
}

std::string Engine::Display() const {
    if (session_ == nullptr) {
        return {};
    }
    const char* text = vfk_display_text(session_);
    return text == nullptr ? std::string() : std::string(text);
}

bool Engine::Enabled() const {
    return session_ != nullptr && vfk_is_enabled(session_);
}

void Engine::Reset() {
    if (session_ != nullptr) {
        vfk_reset(session_);
    }
}

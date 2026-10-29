// ViFastKey addon cho Fcitx5. Dùng C ABI trong rust/crates/vfk-ffi/include/vifastkey.h.
// Logic tiếng Việt nằm ở Rust; file này chỉ ánh xạ KeyEvent/Action và preedit.
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodentry.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputpanel.h>
#include <fcitx/instance.h>
#include <fcitx/text.h>
#include <fcitx/userinterface.h>
#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysym.h>

#include <cstdlib>
#include <cstdint>
#include <fstream>
#include <iterator>
#include <memory>
#include <string>

#include "vifastkey.h"

namespace fcitx {
namespace {

std::string configPath() {
    const char *xdg = std::getenv("XDG_CONFIG_HOME");
    if (xdg && *xdg) {
        return std::string(xdg) + "/vifastkey/config.json";
    }
    const char *home = std::getenv("HOME");
    if (home && *home) {
        return std::string(home) + "/.config/vifastkey/config.json";
    }
    return {};
}

std::string readConfig() {
    std::ifstream in(configPath());
    if (!in) {
        return {};
    }
    return std::string(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
}

// Cắt `chars` code point đầu tiên của chuỗi UTF-8.
std::string utf8Prefix(const std::string &text, unsigned int chars) {
    std::string out;
    unsigned int count = 0;
    for (char c : text) {
        if (count >= chars) {
            break;
        }
        out.push_back(c);
        if ((static_cast<unsigned char>(c) & 0xC0) != 0x80) {
            ++count;
        }
    }
    return out;
}

std::string keyName(KeySym sym, KeyStates states) {
    switch (sym) {
    case FcitxKey_BackSpace: return "Backspace";
    case FcitxKey_Return: return "Enter";
    case FcitxKey_KP_Enter: return "Enter";
    case FcitxKey_Tab: return "Tab";
    case FcitxKey_Escape: return "Escape";
    case FcitxKey_Left: return "Left";
    case FcitxKey_Right: return "Right";
    case FcitxKey_Up: return "Up";
    case FcitxKey_Down: return "Down";
    case FcitxKey_Home: return "Home";
    case FcitxKey_End: return "End";
    case FcitxKey_Delete: return "Delete";
    case FcitxKey_Page_Up: return "PageUp";
    case FcitxKey_Page_Down: return "PageDown";
    default: break;
    }
    std::string text = Key::keySymToUTF8(sym);
    if (text.empty()) {
        return {};
    }
    if (states.test(KeyState::Shift) && text.size() == 1 &&
        text[0] >= 'a' && text[0] <= 'z') {
        text[0] = static_cast<char>(text[0] - 'a' + 'A');
    }
    return text;
}

} // namespace

class VifastkeyEngine;

class VifastkeyState final : public InputContextProperty {
public:
    VifastkeyState(VifastkeyEngine *engine, InputContext *ic)
        : engine_(engine), ic_(ic), config_(readConfig()) {
        session_ = vfk_session_new(config_.empty() ? nullptr : config_.c_str(), VFK_MODE_PREEDIT);
    }
    ~VifastkeyState() override {
        if (session_) {
            vfk_session_free(session_);
        }
    }

    void focus(const std::string &program);
    void blur();
    void reset();
    void keyEvent(KeyEvent &event);

private:
    void refreshSurrounding();
    void setPreedit(const std::string &text);
    void clearPreedit();
    void apply(const VfkAction &action, KeyEvent *event);

    VifastkeyEngine *engine_;
    InputContext *ic_;
    std::string config_;
    VfkSession *session_ = nullptr;
};

class VifastkeyEngine final : public InputMethodEngineV2 {
public:
    explicit VifastkeyEngine(Instance *instance)
        : instance_(instance), factory_([this](InputContext &ic) {
              return new VifastkeyState(this, &ic);
          }) {
        instance_->inputContextManager().registerProperty("vifastkey-state", &factory_);
    }

    void keyEvent(const InputMethodEntry &, KeyEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        state->keyEvent(event);
    }

    void activate(const InputMethodEntry &, InputContextEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        state->focus(event.inputContext()->program());
    }

    void deactivate(const InputMethodEntry &, InputContextEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        state->blur();
        state->reset();
    }

    void reset(const InputMethodEntry &, InputContextEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        state->reset();
    }

private:
    Instance *instance_;
    FactoryFor<VifastkeyState> factory_;
};

void VifastkeyState::focus(const std::string &program) {
    if (!session_) {
        return;
    }
    vfk_set_surrounding_text(session_, nullptr);
    VfkAction action = vfk_focus_changed(session_, program.empty() ? nullptr : program.c_str());
    apply(action, nullptr);
}

void VifastkeyState::blur() {
    if (!session_) {
        return;
    }
    VfkAction action = vfk_focus_changed(session_, nullptr);
    apply(action, nullptr);
}

void VifastkeyState::reset() {
    if (session_) {
        vfk_reset(session_);
    }
    clearPreedit();
}

void VifastkeyState::refreshSurrounding() {
    if (!session_) {
        return;
    }
    if (!ic_->capabilityFlags().test(CapabilityFlag::SurroundingText)) {
        vfk_set_surrounding_text(session_, nullptr);
        return;
    }
    const auto &surrounding = ic_->surroundingText();
    if (!surrounding.isValid()) {
        vfk_set_surrounding_text(session_, nullptr);
        return;
    }
    std::string before = utf8Prefix(surrounding.text(), surrounding.cursor());
    vfk_set_surrounding_text(session_, before.c_str());
}

void VifastkeyState::setPreedit(const std::string &text) {
    auto &panel = ic_->inputPanel();
    panel.reset();
    if (!text.empty()) {
        Text preedit(text);
        preedit.setCursor(static_cast<int>(text.size()));
        if (ic_->capabilityFlags().test(CapabilityFlag::Preedit)) {
            panel.setClientPreedit(preedit);
        } else {
            panel.setPreedit(preedit);
        }
    }
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

void VifastkeyState::clearPreedit() {
    ic_->inputPanel().reset();
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

void VifastkeyState::apply(const VfkAction &action, KeyEvent *event) {
    std::string text = action.text ? action.text : "";
    switch (action.kind) {
    case VFK_PASS:
        return;
    case VFK_PREEDIT:
        setPreedit(text);
        if (event) {
            event->filterAndAccept();
        }
        return;
    case VFK_COMMIT:
        ic_->commitString(text);
        clearPreedit();
        if (event && !action.pass_key) {
            event->filterAndAccept();
        }
        return;
    case VFK_REPLACE:
        if (action.delete_utf16 > 0) {
            ic_->deleteSurroundingText(-static_cast<int>(action.delete_utf16),
                                       action.delete_utf16);
        }
        ic_->commitString(text);
        clearPreedit();
        if (event && !action.pass_key) {
            event->filterAndAccept();
        }
        return;
    default:
        return;
    }
}

void VifastkeyState::keyEvent(KeyEvent &event) {
    if (!session_ || event.isRelease()) {
        return;
    }
    const Key raw = event.rawKey();
    const KeySym sym = raw.sym();
    const KeyStates states = raw.states();
    const bool ctrl = states.test(KeyState::Ctrl);
    const bool alt = states.test(KeyState::Alt);
    const bool meta = states.test(KeyState::Super) || states.test(KeyState::Meta);

    if (ctrl && !alt && !meta && states.test(KeyState::Shift) && sym == FcitxKey_F) {
        VfkAction action = vfk_toggle_vietnamese(session_);
        apply(action, &event);
        return;
    }

    std::string key = keyName(sym, states);
    if (key.empty()) {
        return;
    }
    refreshSurrounding();
    VfkAction action = vfk_process_key(session_, key.c_str(), ctrl, alt, meta);
    apply(action, &event);
}

class VifastkeyFactory final : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override {
        return new VifastkeyEngine(manager->instance());
    }
};

} // namespace fcitx

FCITX_ADDON_FACTORY_V2(vifastkey, fcitx::VifastkeyFactory)

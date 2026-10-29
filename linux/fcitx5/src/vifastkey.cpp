// ViFastKey addon cho Fcitx5.
//
// Logic tiếng Việt nằm trong Rust (C ABI); file này chỉ là lớp adapter:
// InputContext state, action application, field/app policy, surrounding-text
// guard, transaction barrier và status action.
#include <fcitx-utils/log.h>
#include <fcitx-utils/signals.h>
#include <fcitx/action.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/inputmethodentry.h>
#include <fcitx/instance.h>
#include <fcitx/statusarea.h>

#include <memory>
#include <string>

#include "config_store.h"
#include "vifastkey_state.h"

namespace fcitx {

class VifastkeyEngine final : public InputMethodEngineV2 {
public:
    explicit VifastkeyEngine(Instance *instance)
        : instance_(instance), config_(std::make_shared<ConfigStore>()),
          factory_([this](InputContext &ic) {
              return new VifastkeyState(instance_->eventLoop(), &ic, config_);
          }),
          toggleAction_(std::make_unique<SimpleAction>()) {
        if (!instance_->inputContextManager().registerProperty(
                "vifastkey-state", &factory_)) {
            FCITX_ERROR() << "vifastkey: cannot register input context state";
        }
        toggleAction_->setIcon("input-keyboard");
        toggleAction_->setShortText("Việt");
        toggleAction_->setLongText("ViFastKey: bật/tắt tiếng Việt");
        toggleConnection_ =
            toggleAction_->connect<SimpleAction::Activated>(
                [this](InputContext *ic) {
                    if (!ic) {
                        return;
                    }
                    auto *state = ic->propertyFor(&factory_);
                    if (!state || state->busy()) {
                        return;
                    }
                    state->toggleVietnamese();
                    updateToggleAction(ic, state);
                });
    }

    void keyEvent(const InputMethodEntry &, KeyEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        if (!state) {
            return;
        }
        state->keyEvent(event);
        updateToggleAction(event.inputContext(), state);
    }

    void activate(const InputMethodEntry &, InputContextEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        if (!state) {
            return;
        }
        state->activate(event.inputContext()->program());
        event.inputContext()->statusArea().addAction(
            StatusGroup::InputMethod, toggleAction_.get());
        updateToggleAction(event.inputContext(), state);
    }

    void deactivate(const InputMethodEntry &,
                    InputContextEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        if (!state) {
            return;
        }
        event.inputContext()->statusArea().removeAction(toggleAction_.get());
        state->deactivate(event);
    }

    void reset(const InputMethodEntry &, InputContextEvent &event) override {
        auto *state = event.inputContext()->propertyFor(&factory_);
        if (!state) {
            return;
        }
        state->reset();
        updateToggleAction(event.inputContext(), state);
    }

private:
    void updateToggleAction(InputContext *ic, VifastkeyState *state) {
        if (!ic || !state) {
            return;
        }
        const bool vietnamese = state->enabled();
        const std::string label = vietnamese ? "Việt" : "Anh";
        if (toggleAction_->shortText(ic) == label &&
            toggleAction_->isChecked(ic) == vietnamese) {
            return;
        }
        toggleAction_->setShortText(label);
        toggleAction_->setChecked(vietnamese);
        toggleAction_->update(ic);
    }

    Instance *instance_;
    std::shared_ptr<ConfigStore> config_;
    FactoryFor<VifastkeyState> factory_;
    std::unique_ptr<SimpleAction> toggleAction_;
    ScopedConnection toggleConnection_;
};

class VifastkeyFactory final : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override {
        return new VifastkeyEngine(manager->instance());
    }
};

} // namespace fcitx

FCITX_ADDON_FACTORY(fcitx::VifastkeyFactory)

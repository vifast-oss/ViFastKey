#include "vifastkey_state.h"

#include <algorithm>
#include <cstdint>
#include <utility>

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/keysym.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/textformatflags.h>
#include <fcitx-utils/utf8.h>
#include <fcitx/inputpanel.h>
#include <fcitx/text.h>
#include <fcitx/userinterface.h>

namespace fcitx {
namespace {

// Khoảng bảo vệ sau Replace để app kịp xử lý commit trước khi key kế tiếp
// bắt đầu một delete/commit mới. Giá trị này khớp
// vfk_adapter_core::timing::POST_COMMIT_DELAY_MS.
constexpr uint32_t kPostReplaceCommitDelayMs = 10;

// Cắt `chars` code point đầu tiên của chuỗi UTF-8.
// Offset được tính bởi adapter-core qua C ABI để C++ không lặp thuật toán.
std::string utf8Prefix(const std::string &text, unsigned int chars) {
    const uint32_t offset = vfk_char_to_byte_offset(text.c_str(), chars);
    return text.substr(0, offset == UINT32_MAX ? text.size() : offset);
}

size_t charOffsetToByte(const std::string &text, unsigned int chars) {
    const uint32_t offset = vfk_char_to_byte_offset(text.c_str(), chars);
    return offset == UINT32_MAX ? text.size() : offset;
}

size_t utf16ToByteOffset(const std::string &text, uint32_t cursorUtf16) {
    const uint32_t offset = vfk_utf16_to_byte_offset(text.c_str(), cursorUtf16);
    return offset == UINT32_MAX ? text.size() : offset;
}

} // namespace

VifastkeyState::VifastkeyState(EventLoop &eventLoop, InputContext *inputContext,
                               ConfigPtr config)
    : eventLoop_(eventLoop), ic_(inputContext), config_(std::move(config)) {
    if (!ic_) {
        return;
    }
    const std::string json = config_ ? config_->json() : std::string();
    session_ = vfk_session_new(json.empty() ? nullptr : json.c_str(),
                               VFK_MODE_PREEDIT);
    if (!session_ && !json.empty()) {
        FCITX_WARN() << "vifastkey: invalid config at startup, using defaults";
        session_ = vfk_session_new(nullptr, VFK_MODE_PREEDIT);
    }
    configGeneration_ = config_ ? config_->generation() : 0;
    if (!session_) {
        FCITX_ERROR() << "vifastkey: cannot create engine session";
    }
}

VifastkeyState::~VifastkeyState() {
    replaceTimer_.reset();
    postCommitTimer_.reset();
    queue_.clear();
    if (session_) {
        vfk_session_free(session_);
        session_ = nullptr;
    }
}

std::string VifastkeyState::keyName(KeySym sym, KeyStates states) {
    switch (sym) {
    case FcitxKey_BackSpace:
        return "Backspace";
    case FcitxKey_Return:
    case FcitxKey_KP_Enter:
        return "Enter";
    case FcitxKey_Tab:
        return "Tab";
    case FcitxKey_Escape:
        return "Escape";
    case FcitxKey_Left:
        return "Left";
    case FcitxKey_Right:
        return "Right";
    case FcitxKey_Up:
        return "Up";
    case FcitxKey_Down:
        return "Down";
    case FcitxKey_Home:
        return "Home";
    case FcitxKey_End:
        return "End";
    case FcitxKey_Delete:
        return "Delete";
    case FcitxKey_Page_Up:
        return "PageUp";
    case FcitxKey_Page_Down:
        return "PageDown";
    default:
        break;
    }

    std::string text = Key::keySymToUTF8(sym);
    if (text.empty()) {
        return {};
    }
    if (states.test(KeyState::Shift) && text.size() == 1 && text[0] >= 'a' &&
        text[0] <= 'z') {
        text[0] = static_cast<char>(text[0] - 'a' + 'A');
    }
    return text;
}

std::string VifastkeyState::displayText() const {
    if (!session_) {
        return {};
    }
    const char *text = vfk_display_text(session_);
    return text ? std::string(text) : std::string();
}

bool VifastkeyState::enabled() const {
    return session_ && vfk_is_enabled(session_);
}

bool VifastkeyState::hasComposition() const {
    return hasPreedit_ || !displayText().empty();
}

bool VifastkeyState::isDisabled() const {
    return disabled_ || fieldKind_ == VFK_FIELD_PASSWORD ||
           fieldKind_ == VFK_FIELD_NUMBER || fieldKind_ == VFK_FIELD_EMAIL;
}

bool VifastkeyState::hasSurrounding() const {
    return ic_ &&
           ic_->capabilityFlags().test(CapabilityFlag::SurroundingText) &&
           ic_->surroundingText().isValid() &&
           ic_->surroundingText().cursor() == ic_->surroundingText().anchor();
}

void VifastkeyState::refreshSurrounding() {
    if (!session_) {
        return;
    }
    if (!ic_ ||
        !ic_->capabilityFlags().test(CapabilityFlag::SurroundingText)) {
        vfk_set_surrounding_text(session_, nullptr);
        return;
    }

    const auto &surrounding = ic_->surroundingText();
    // Selection đang mở là ngữ cảnh không an toàn cho edit-previous-word.
    // Không đưa surrounding vào engine để tránh adopt nhầm.
    if (!surrounding.isValid() ||
        surrounding.cursor() != surrounding.anchor()) {
        vfk_set_surrounding_text(session_, nullptr);
        return;
    }

    const std::string &text = surrounding.text();
    const unsigned int cursor = surrounding.cursor();
    // Nếu caret đang ở giữa một từ (ký tự ngay sau caret vẫn là chữ), không
    // đưa surrounding vào engine: adopt một prefix cụt sẽ sửa sai phần còn lại.
    if (cursor < fcitx::utf8::length(text)) {
        const size_t byteOffset = charOffsetToByte(text, cursor);
        uint32_t nextCodepoint = 0;
        fcitx::utf8::getNextChar(
            text.begin() + static_cast<std::ptrdiff_t>(byteOffset), text.end(),
            &nextCodepoint);
        if (vfk_is_word_codepoint(nextCodepoint)) {
            vfk_set_surrounding_text(session_, nullptr);
            return;
        }
    }

    const std::string before = utf8Prefix(text, cursor);
    vfk_set_surrounding_text(session_, before.c_str());
}

void VifastkeyState::syncConfig(bool force) {
    if (!session_ || !config_) {
        return;
    }
    config_->refresh(force);
    if (config_->generation() == configGeneration_) {
        return;
    }
    if (!force && !displayText().empty()) {
        // Đang có composition: để lượt sau, tránh reload làm mất từ.
        return;
    }

    const std::string before = displayText();
    const std::string &json = config_->json();
    VfkAction action = {};
    const bool ok =
        vfk_apply_config_json(session_, json.empty() ? "{}" : json.c_str(),
                              &action);
    configGeneration_ = config_->generation();
    if (ok) {
        applyAction(action, nullptr, before);
    } else {
        FCITX_WARN() << "vifastkey: invalid config JSON, keeping previous "
                        "config path="
                     << config_->path();
    }
}

void VifastkeyState::syncUserWords() {
    if (!session_ || !config_) {
        return;
    }
    if (config_->userWordsGeneration() == userWordsGeneration_) {
        return;
    }
    const std::string &words = config_->userWords();
    if (!words.empty()) {
        vfk_add_dictionary_words(session_, words.c_str(), true);
    }
    userWordsGeneration_ = config_->userWordsGeneration();
}

void VifastkeyState::syncProgram() {
    if (!session_ || !ic_) {
        return;
    }
    const std::string &current = ic_->program();
    if (current == program_ || !displayText().empty()) {
        return;
    }
    const std::string before = displayText();
    const VfkAction action =
        vfk_focus_changed(session_, current.empty() ? nullptr : current.c_str());
    program_ = current;
    applyAction(action, nullptr, before);
}

void VifastkeyState::syncFieldKind() {
    if (!session_ || !ic_) {
        return;
    }

    const auto flags = ic_->capabilityFlags();
    const bool disabled = flags.test(CapabilityFlag::Disable);
    const bool isUrl = flags.test(CapabilityFlag::Url);
    uint32_t kind = VFK_FIELD_TEXT;
    if (disabled || flags.test(CapabilityFlag::Password) ||
        (flags.test(CapabilityFlag::Sensitive) && !isUrl)) {
        kind = VFK_FIELD_PASSWORD;
    } else if (isUrl) {
        // Chrome/Chromium omnibox is a URL *and* search box. Keep the URL kind
        // so the engine can enable URL-safe transforms (no macros/quick telex),
        // but do not disable the field: Vietnamese search must work.
        kind = VFK_FIELD_URL;
    } else if (flags.test(CapabilityFlag::Email)) {
        kind = VFK_FIELD_EMAIL;
    } else if (flags.test(CapabilityFlag::Number) ||
               flags.test(CapabilityFlag::Digit) ||
               flags.test(CapabilityFlag::Dialable)) {
        kind = VFK_FIELD_NUMBER;
    } else if (flags.test(CapabilityFlag::Terminal)) {
        kind = VFK_FIELD_TERMINAL;
    }

    if (kind == fieldKind_ && disabled == disabled_) {
        return;
    }

    FCITX_DEBUG() << "vifastkey: field kind " << fieldKind_ << " -> " << kind
                  << " password=" << flags.test(CapabilityFlag::Password)
                  << " sensitive=" << flags.test(CapabilityFlag::Sensitive)
                  << " url=" << isUrl
                  << " email=" << flags.test(CapabilityFlag::Email)
                  << " number=" << flags.test(CapabilityFlag::Number)
                  << " terminal=" << flags.test(CapabilityFlag::Terminal);

    const bool wasDisabled = isDisabled();
    const bool nowDisabled =
        disabled || kind == VFK_FIELD_PASSWORD || kind == VFK_FIELD_NUMBER ||
        kind == VFK_FIELD_EMAIL;

    const std::string before = displayText();
    const VfkAction action = vfk_set_field_kind(session_, kind);
    fieldKind_ = kind;
    disabled_ = disabled;

    if (nowDisabled && !wasDisabled) {
        // Không commit composition cũ vào password/number/email: tránh lộ chữ
        // và tránh chèn nhầm vào field mới.
        vfk_reset(session_);
        clearPreedit();
        FCITX_INFO() << "vifastkey: field became disabled, composition dropped";
        return;
    }

    applyAction(action, nullptr, before);
    if (disabled_) {
        FCITX_INFO() << "vifastkey: field disabled by client flags";
    }
}

void VifastkeyState::syncProfileMode() {
    if (!session_ || isDisabled()) {
        return;
    }
    const uint32_t wanted = vfk_profile_output_mode(session_);
    const bool wantReplace = wanted == 2 && hasSurrounding();
    const uint32_t target = wantReplace ? VFK_MODE_REPLACE : VFK_MODE_PREEDIT;
    if (target == mode_) {
        return;
    }

    const std::string before = displayText();
    const VfkAction action = vfk_set_output_mode(session_, target);
    mode_ = target;
    applyAction(action, nullptr, before);
}

void VifastkeyState::syncContext(bool forceConfig) {
    syncConfig(forceConfig);
    syncUserWords();
    syncProgram();
    syncFieldKind();
    if (!isDisabled()) {
        syncProfileMode();
    }
}

void VifastkeyState::activate(const std::string &program) {
    if (!session_) {
        return;
    }
    syncConfig(true);
    syncUserWords();
    const std::string before = displayText();
    const VfkAction action =
        vfk_focus_changed(session_, program.empty() ? nullptr : program.c_str());
    program_ = program;
    applyAction(action, nullptr, before);
    syncFieldKind();
    syncProfileMode();
}

void VifastkeyState::deactivate(InputContextEvent &event) {
    if (!session_) {
        return;
    }

    // Fcitx core already commits client preedit on focus-out (unless the
    // client opted into ClientUnfocusCommit). Committing again here would
    // duplicate the word.
    const bool focusOut = event.type() == EventType::InputContextFocusOut;
    queue_.clear();
    postCommitTimer_.reset();
    postCommitPending_ = false;
    if (busy_) {
        finishReplace();
        postCommitTimer_.reset();
        postCommitPending_ = false;
    }

    if (focusOut && usingClientPreedit_ && hasPreedit_) {
        vfk_reset(session_);
        clearPreedit();
        usingClientPreedit_ = false;
        hasPreedit_ = false;
        return;
    }

    const std::string before = displayText();
    const VfkAction action = vfk_flush(session_);
    applyAction(action, nullptr, before);
    vfk_reset(session_);
    clearPreedit();
    usingClientPreedit_ = false;
    hasPreedit_ = false;
}

void VifastkeyState::reset() {
    if (!session_) {
        return;
    }
    queue_.clear();
    postCommitTimer_.reset();
    postCommitPending_ = false;
    if (busy_) {
        finishReplace();
        postCommitTimer_.reset();
        postCommitPending_ = false;
    }

    const std::string before = displayText();
    const VfkAction action = vfk_flush(session_);
    applyAction(action, nullptr, before);
    vfk_reset(session_);
    clearPreedit();
    usingClientPreedit_ = false;
    hasPreedit_ = false;
}

void VifastkeyState::toggleVietnamese() {
    if (!session_ || isDisabled()) {
        return;
    }
    const std::string before = displayText();
    const VfkAction action = vfk_toggle_vietnamese(session_);
    applyAction(action, nullptr, before);
}

void VifastkeyState::keyEvent(KeyEvent &event) {
    if (!session_ || event.isRelease()) {
        return;
    }

    const Key raw = event.rawKey();
    const KeyStates states = raw.states();
    const bool ctrl = states.test(KeyState::Ctrl);
    const bool alt = states.test(KeyState::Alt);
    const bool meta =
        states.test(KeyState::Super) || states.test(KeyState::Meta);
    const std::string key = keyName(raw.sym(), states);
    if (key.empty()) {
        return;
    }

    if (busy_ || postCommitPending_) {
        // Key đã bị chặn khỏi app; giữ nguyên thứ tự để xử lý sau transaction.
        // Không syncContext ở đây vì field/config change có thể flush state
        // trong lúc delete/commit đang chờ.
        event.filterAndAccept();
        queue_.push_back(QueuedKey{key, ctrl, alt, meta, raw});
        return;
    }

    syncContext(false);
    if (isDisabled()) {
        return;
    }
    processKey(key, ctrl, alt, meta, &event, raw);
}

void VifastkeyState::processKey(const std::string &key, bool ctrl, bool alt,
                                bool meta, KeyEvent *event,
                                const std::optional<Key> &rawKey) {
    if (!session_) {
        return;
    }

    refreshSurrounding();

    if (rawKey && alt && !ctrl && !meta &&
        !rawKey->states().test(KeyState::Shift) &&
        rawKey->sym() == FcitxKey_comma) {
        toggleVietnamese();
        if (event) {
            event->filterAndAccept();
        }
        return;
    }

    std::string before = displayText();
    if (before.empty() && hasSurrounding()) {
        const auto &surrounding = ic_->surroundingText();
        before = utf8Prefix(surrounding.text(), surrounding.cursor());
    }
    const VfkAction action =
        vfk_process_key(session_, key.c_str(), ctrl, alt, meta);
    applyAction(action, event, before, rawKey);
}

void VifastkeyState::applyPreedit(const std::string &text,
                                 uint32_t cursorUtf16, KeyEvent *event) {
    if (!ic_) {
        return;
    }

    auto &panel = ic_->inputPanel();
    panel.reset();
    usingClientPreedit_ = false;
    hasPreedit_ = !text.empty();

    if (!text.empty()) {
        TextFormatFlags format = TextFormatFlag::Underline;
        Text preedit(text, format);
        const size_t cursor = std::min<size_t>(
            utf16ToByteOffset(text, cursorUtf16), text.size());
        preedit.setCursor(static_cast<int>(cursor));
        usingClientPreedit_ =
            ic_->capabilityFlags().test(CapabilityFlag::Preedit);
        if (usingClientPreedit_) {
            panel.setClientPreedit(preedit);
        } else {
            panel.setPreedit(preedit);
        }
    }

    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
    if (event) {
        event->filterAndAccept();
    }
}

void VifastkeyState::applyCommit(const std::string &text, bool passKey,
                                KeyEvent *event,
                                const std::optional<Key> &queuedKey) {
    if (!ic_) {
        return;
    }
    if (!text.empty()) {
        ic_->commitString(text);
        updateSurroundingAfterCommit(text);
    }
    clearPreedit();

    if (event && !passKey) {
        event->filterAndAccept();
    }
    if (!event && passKey && queuedKey) {
        forwardQueuedKey(*queuedKey);
    }
}

void VifastkeyState::failSafe(KeyEvent *event,
                             const std::optional<Key> &queuedKey) {
    FCITX_WARN() << "vifastkey: plan fail-safe, not touching surrounding text";
    // Không xoá mù. Nếu đang PREEDIT, chữ người dùng đã gõ chỉ nằm trong
    // composition; commit nó trước khi reset để không mất từ. Ở REPLACE,
    // chữ đã nằm trên màn hình nên không commit lại (tránh nhân đôi).
    const std::string text = displayText();
    if (!text.empty() && mode_ == VFK_MODE_PREEDIT && ic_) {
        ic_->commitString(text);
        updateSurroundingAfterCommit(text);
    }
    if (session_) {
        vfk_reset(session_);
    }
    clearPreedit();
    // Event đang xử lý không bị filter: phím gốc vẫn tới app sau khi chốt từ.
    if (!event && queuedKey) {
        forwardQueuedKey(*queuedKey);
    }
}

void VifastkeyState::applyAction(const VfkAction &action, KeyEvent *event,
                                const std::string &beforeDisplay,
                                const std::optional<Key> &queuedKey) {
    if (!session_) {
        return;
    }

    // Gom đúng những gì adapter-core cần: display trước phím + snapshot đầy
    // đủ. Nếu app không hỗ trợ surrounding, truyền nullptr để planner fail-safe.
    std::string surroundingText;
    bool hasSnapshot = false;
    uint32_t cursorUtf16 = 0;
    uint32_t anchorUtf16 = 0;
    if (ic_ &&
        ic_->capabilityFlags().test(CapabilityFlag::SurroundingText)) {
        const auto &surrounding = ic_->surroundingText();
        if (surrounding.isValid()) {
            surroundingText = surrounding.text();
            const uint32_t c = vfk_char_to_utf16_offset(
                surroundingText.c_str(), surrounding.cursor());
            const uint32_t a = vfk_char_to_utf16_offset(
                surroundingText.c_str(), surrounding.anchor());
            if (c != UINT32_MAX && a != UINT32_MAX) {
                hasSnapshot = true;
                cursorUtf16 = c;
                anchorUtf16 = a;
            }
        }
    }

    VfkPlan plan = {};
    const uint32_t fallback = hasSnapshot ? VFK_FALLBACK_STALE_SURROUNDING
                                          : VFK_FALLBACK_NONE;
    const bool planned = vfk_plan_action_with_fallback(
        session_, &action, fieldKind_, beforeDisplay.c_str(),
        hasSnapshot ? surroundingText.c_str() : nullptr,
        cursorUtf16, anchorUtf16, false, fallback, &plan);
    if (!planned) {
        failSafe(event, queuedKey);
        return;
    }

    const std::string text = plan.text ? plan.text : "";
    switch (plan.kind) {
    case VFK_PLAN_PASS:
        if (event) {
            return; // để app xử lý phím gốc
        }
        if (queuedKey) {
            forwardQueuedKey(*queuedKey);
        }
        return;

    case VFK_PLAN_PREEDIT:
        applyPreedit(text, plan.cursor_utf16, event);
        return;

    case VFK_PLAN_COMMIT:
        applyCommit(text, plan.pass_key, event, queuedKey);
        return;

    case VFK_PLAN_REPLACE:
        if (plan.delete_chars == 0) {
            applyCommit(text, plan.pass_key, event, queuedKey);
            return;
        }
        startReplace(text, static_cast<int>(plan.delete_chars), plan.pass_key,
                     queuedKey, event, plan.wait_ms);
        return;

    case VFK_PLAN_FAIL_SAFE:
    default:
        failSafe(event, queuedKey);
        return;
    }
}

void VifastkeyState::startReplace(const std::string &text, int deleteChars,
                                 bool passKey,
                                 const std::optional<Key> &queuedKey,
                                 KeyEvent *event, uint32_t waitMs) {
    if (!ic_ || busy_) {
        return;
    }
    busy_ = true;
    pendingText_ = text;
    pendingPassKey_ = passKey;
    pendingRawKey_ = queuedKey;

    ic_->deleteSurroundingText(-deleteChars, deleteChars);
    updateSurroundingAfterDelete(deleteChars);

    // Phím gốc được chặn ngay; nếu pass_key=true nó sẽ được forward sau khi
    // commit xong để giữ đúng thứ tự xoá → chèn → phím gốc.
    if (event) {
        event->filterAndAccept();
    }

    scheduleReplaceCommit(waitMs);
}

void VifastkeyState::scheduleReplaceCommit(uint32_t waitMs) {
    replaceTimer_.reset();
    const uint64_t deadline =
        now(CLOCK_MONOTONIC) + static_cast<uint64_t>(waitMs) * 1000;
    replaceTimer_ = eventLoop_.addTimeEvent(
        CLOCK_MONOTONIC, deadline, 1000,
        [this](EventSourceTime *, uint64_t) {
            auto completed = std::move(replaceTimer_);
            (void)completed;
            finishReplace();
            return false;
        });
    if (replaceTimer_) {
        replaceTimer_->setOneShot();
    } else {
        finishReplace();
    }
}

void VifastkeyState::finishReplace() {
    if (!busy_) {
        return;
    }
    busy_ = false;
    replaceTimer_.reset();

    if (ic_) {
        if (!pendingText_.empty()) {
            ic_->commitString(pendingText_);
            updateSurroundingAfterCommit(pendingText_);
        }
        clearPreedit();
        if (pendingPassKey_ && pendingRawKey_) {
            forwardQueuedKey(*pendingRawKey_);
        }
    }

    pendingText_.clear();
    pendingPassKey_ = false;
    pendingRawKey_.reset();
    schedulePostCommitDrain();
}

void VifastkeyState::schedulePostCommitDrain() {
    postCommitTimer_.reset();
    postCommitPending_ = true;
    const uint64_t deadline =
        now(CLOCK_MONOTONIC) +
        static_cast<uint64_t>(kPostReplaceCommitDelayMs) * 1000;
    postCommitTimer_ = eventLoop_.addTimeEvent(
        CLOCK_MONOTONIC, deadline, 1000,
        [this](EventSourceTime *, uint64_t) {
            auto completed = std::move(postCommitTimer_);
            (void)completed;
            postCommitPending_ = false;
            drainQueue();
            return false;
        });
    if (postCommitTimer_) {
        postCommitTimer_->setOneShot();
    } else {
        postCommitPending_ = false;
        drainQueue();
    }
}

void VifastkeyState::drainQueue() {
    while (!busy_ && !postCommitPending_ && !queue_.empty()) {
        QueuedKey queued = std::move(queue_.front());
        queue_.pop_front();

        // Field/app có thể đã đổi trong lúc transaction chạy. Đồng bộ lại
        // trước khi xử lý key đã queue; nếu field chuyển sang password/URL,
        // forward key gốc thay vì đưa vào engine.
        syncContext(false);
        if (isDisabled()) {
            forwardQueuedKey(queued.rawKey);
            continue;
        }
        processKey(queued.key, queued.ctrl, queued.alt, queued.meta, nullptr,
                   queued.rawKey);
    }
}

void VifastkeyState::updateSurroundingAfterDelete(int deleteChars) {
    if (!ic_) {
        return;
    }
    auto &surrounding = ic_->surroundingText();
    if (!surrounding.isValid() || deleteChars <= 0) {
        return;
    }
    if (static_cast<unsigned int>(deleteChars) > surrounding.cursor()) {
        surrounding.invalidate();
        return;
    }
    surrounding.deleteText(-deleteChars, static_cast<unsigned int>(deleteChars));
}

void VifastkeyState::updateSurroundingAfterCommit(const std::string &insert) {
    if (!ic_) {
        return;
    }
    auto &surrounding = ic_->surroundingText();
    if (!surrounding.isValid()) {
        return;
    }
    const std::string text = surrounding.text();
    const unsigned int cursor = surrounding.cursor();
    const size_t byteOffset = charOffsetToByte(text, cursor);
    if (byteOffset > text.size()) {
        surrounding.invalidate();
        return;
    }
    std::string updated = text.substr(0, byteOffset);
    updated += insert;
    updated += text.substr(byteOffset);
    const unsigned int newCursor =
        cursor + static_cast<unsigned int>(fcitx::utf8::length(insert));
    surrounding.setText(updated, newCursor, newCursor);
}

void VifastkeyState::clearPreedit() {
    if (!ic_) {
        return;
    }
    ic_->inputPanel().reset();
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
    usingClientPreedit_ = false;
    hasPreedit_ = false;
}

void VifastkeyState::forwardQueuedKey(const Key &key) {
    if (!ic_) {
        return;
    }
    ic_->forwardKey(key, false);
    ic_->forwardKey(key, true);
}

} // namespace fcitx

#include "vifastkey_state.h"

#include <algorithm>
#include <cstdint>
#include <utility>

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/keysym.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/utf8.h>
#include <fcitx/inputpanel.h>
#include <fcitx/text.h>
#include <fcitx/userinterface.h>

namespace fcitx {
namespace {

constexpr uint32_t kMaxReplaceWaitMs = 60;
constexpr uint32_t kMinReplaceWaitMs = 3;
// Khoảng bảo vệ sau Replace để app kịp xử lý commit trước khi key kế tiếp
// bắt đầu một delete/commit mới.
constexpr uint32_t kPostReplaceCommitDelayMs = 10;

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

size_t charOffsetToByte(const std::string &text, unsigned int chars) {
    size_t offset = 0;
    unsigned int count = 0;
    while (offset < text.size() && count < chars) {
        ++offset;
        while (offset < text.size() &&
               (static_cast<unsigned char>(text[offset]) & 0xC0) == 0x80) {
            ++offset;
        }
        ++count;
    }
    return offset;
}

uint32_t utf16Units(uint32_t codepoint) {
    return codepoint > 0xFFFF ? 2 : 1;
}

std::string utf16Suffix(const std::string &text, uint32_t units) {
    if (units == 0) {
        return {};
    }
    size_t begin = text.size();
    uint32_t count = 0;
    while (begin > 0 && count < units) {
        size_t start = begin - 1;
        while (start > 0 &&
               (static_cast<unsigned char>(text[start]) & 0xC0) == 0x80) {
            --start;
        }
        uint32_t codepoint = 0;
        fcitx::utf8::getNextChar(text.begin() + start, text.end(), &codepoint);
        const uint32_t add = utf16Units(codepoint);
        if (count + add > units) {
            return {};
        }
        count += add;
        begin = start;
    }
    if (count != units) {
        return {};
    }
    return text.substr(begin);
}

size_t utf16ToByteOffset(const std::string &text, uint32_t cursorUtf16) {
    size_t offset = 0;
    uint32_t units = 0;
    while (offset < text.size() && units < cursorUtf16) {
        uint32_t codepoint = 0;
        auto next = fcitx::utf8::getNextChar(
            text.begin() + static_cast<std::ptrdiff_t>(offset), text.end(),
            &codepoint);
        units += utf16Units(codepoint);
        offset = static_cast<size_t>(next - text.begin());
    }
    return offset;
}

bool endsWith(const std::string &text, const std::string &suffix) {
    return suffix.size() <= text.size() &&
           text.compare(text.size() - suffix.size(), suffix.size(), suffix) ==
               0;
}

bool isWordCodepoint(uint32_t codepoint) {
    if (codepoint < 0x80) {
        return (codepoint >= '0' && codepoint <= '9') ||
               (codepoint >= 'A' && codepoint <= 'Z') ||
               (codepoint >= 'a' && codepoint <= 'z') || codepoint == '_';
    }
    // Latin mở rộng + khối tiếng Việt + combining marks.
    return (codepoint >= 0x00C0 && codepoint <= 0x024F) ||
           (codepoint >= 0x0300 && codepoint <= 0x036F) ||
           (codepoint >= 0x1E00 && codepoint <= 0x1EFF);
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
           fieldKind_ == VFK_FIELD_NUMBER || fieldKind_ == VFK_FIELD_EMAIL ||
           fieldKind_ == VFK_FIELD_URL;
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
        if (isWordCodepoint(nextCodepoint)) {
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
    bool disabled = flags.test(CapabilityFlag::Disable);
    uint32_t kind = VFK_FIELD_TEXT;
    if (disabled || flags.test(CapabilityFlag::Password) ||
        flags.test(CapabilityFlag::Sensitive)) {
        kind = VFK_FIELD_PASSWORD;
    } else if (flags.test(CapabilityFlag::Email)) {
        kind = VFK_FIELD_EMAIL;
    } else if (flags.test(CapabilityFlag::Url)) {
        kind = VFK_FIELD_URL;
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

    const bool wasDisabled = isDisabled();
    const bool nowDisabled =
        disabled || kind == VFK_FIELD_PASSWORD || kind == VFK_FIELD_NUMBER ||
        kind == VFK_FIELD_EMAIL || kind == VFK_FIELD_URL;

    const std::string before = displayText();
    const VfkAction action = vfk_set_field_kind(session_, kind);
    fieldKind_ = kind;
    disabled_ = disabled;

    if (nowDisabled && !wasDisabled) {
        // Không commit composition cũ vào password/URL/number: tránh lộ chữ
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

    if (rawKey && ctrl && !alt && !meta &&
        rawKey->states().test(KeyState::Shift) &&
        rawKey->sym() == FcitxKey_F) {
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
        Text preedit(text);
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

void VifastkeyState::applyAction(const VfkAction &action, KeyEvent *event,
                                const std::string &beforeDisplay,
                                const std::optional<Key> &queuedKey) {
    if (!session_) {
        return;
    }

    const std::string text = action.text ? action.text : "";
    switch (action.kind) {
    case VFK_PASS:
        if (event) {
            return; // để app xử lý phím gốc
        }
        if (queuedKey) {
            forwardQueuedKey(*queuedKey);
        }
        return;

    case VFK_PREEDIT:
        applyPreedit(text, action.cursor_utf16, event);
        return;

    case VFK_COMMIT:
        applyCommit(text, action.pass_key, event, queuedKey);
        return;

    case VFK_REPLACE: {
        if (action.delete_utf16 == 0) {
            applyCommit(text, action.pass_key, event, queuedKey);
            return;
        }

        const std::string deleted = utf16Suffix(beforeDisplay, action.delete_utf16);
        if (deleted.empty() || !safeToReplace(deleted)) {
            // Snapshot lệch hoặc app không còn surrounding text. Không xoá mù:
            // reset engine và để phím đi tiếp để tránh làm hỏng văn bản.
            FCITX_WARN() << "vifastkey: surrounding text mismatch, fail-safe "
                            "replace";
            vfk_reset(session_);
            clearPreedit();
            if (!event && queuedKey) {
                forwardQueuedKey(*queuedKey);
            }
            return;
        }

        const int deleteChars = static_cast<int>(fcitx::utf8::length(deleted));
        if (deleteChars <= 0) {
            applyCommit(text, action.pass_key, event, queuedKey);
            return;
        }
        startReplace(text, deleteChars, action.pass_key, queuedKey, event);
        return;
    }

    default:
        return;
    }
}

bool VifastkeyState::safeToReplace(const std::string &deleted) const {
    if (!ic_ ||
        !ic_->capabilityFlags().test(CapabilityFlag::SurroundingText)) {
        return false;
    }
    const auto &surrounding = ic_->surroundingText();
    if (!surrounding.isValid() ||
        surrounding.cursor() != surrounding.anchor()) {
        return false;
    }
    const std::string before =
        utf8Prefix(surrounding.text(), surrounding.cursor());
    return endsWith(before, deleted);
}

uint32_t VifastkeyState::replaceWaitMs(int deleteChars) const {
    if (!session_) {
        return kMinReplaceWaitMs;
    }
    uint32_t wait = kMinReplaceWaitMs;
    if (vfk_profile_wait_ack(session_)) {
        const uint32_t perChar = vfk_profile_delay_per_char_ms(session_);
        const uint32_t base = vfk_profile_backspace_delay_ms(session_);
        const uint64_t computed = static_cast<uint64_t>(base) +
                                  static_cast<uint64_t>(perChar) *
                                      static_cast<uint64_t>(deleteChars);
        wait = static_cast<uint32_t>(
            std::min<uint64_t>(kMaxReplaceWaitMs, std::max<uint64_t>(wait, computed)));
    }
    return wait;
}

void VifastkeyState::startReplace(const std::string &text, int deleteChars,
                                 bool passKey,
                                 const std::optional<Key> &queuedKey,
                                 KeyEvent *event) {
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

    scheduleReplaceCommit(replaceWaitMs(deleteChars));
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

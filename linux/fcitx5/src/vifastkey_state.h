#pragma once

#include <cstdint>
#include <deque>
#include <memory>
#include <optional>
#include <string>

#include <fcitx-utils/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>

#include "config_store.h"
#include "vifastkey.h"

namespace fcitx {

// Trạng thái engine cho một InputContext.
//
// Adapter giữ PREEDIT làm mặc định. REPLACE chỉ dùng cho:
//   * sửa từ đã commit (engine tự chọn khi surrounding text khớp),
//   * macro undo,
//   * app profile yêu cầu output_mode=replace và app có surrounding text.
//
// Mọi thao tác REPLACE đi qua một transaction ngắn: xoá surrounding text,
// chờ frontend xử lý, rồi commit. Key đến trong lúc transaction đang chạy được
// xếp hàng để không làm lệch thứ tự.
class VifastkeyState final : public InputContextProperty {
public:
    using ConfigPtr = std::shared_ptr<ConfigStore>;

    VifastkeyState(EventLoop &eventLoop, InputContext *inputContext,
                   ConfigPtr config);
    ~VifastkeyState() override;

    void activate(const std::string &program);
    void deactivate(InputContextEvent &event);
    void reset();
    void keyEvent(KeyEvent &event);
    void toggleVietnamese();

    bool enabled() const;
    bool hasComposition() const;
    bool busy() const { return busy_; }

    // Public để test adapter trực tiếp với VfkAction tổng hợp.
    void applyAction(const VfkAction &action, KeyEvent *event,
                     const std::string &beforeDisplay,
                     const std::optional<Key> &queuedKey = std::nullopt);

private:
    struct QueuedKey {
        std::string key;
        bool ctrl = false;
        bool alt = false;
        bool meta = false;
        Key rawKey;
    };

    static std::string keyName(KeySym sym, KeyStates states);

    std::string displayText() const;
    void refreshSurrounding();
    void syncConfig(bool force);
    void syncProgram();
    void syncFieldKind();
    void syncProfileMode();
    void syncContext(bool forceConfig = false);

    void processKey(const std::string &key, bool ctrl, bool alt, bool meta,
                    KeyEvent *event, const std::optional<Key> &rawKey);
    void applyPreedit(const std::string &text, uint32_t cursorUtf16,
                      KeyEvent *event);
    void applyCommit(const std::string &text, bool passKey, KeyEvent *event,
                     const std::optional<Key> &queuedKey);
    void startReplace(const std::string &text, int deleteChars, bool passKey,
                      const std::optional<Key> &rawKey, KeyEvent *event);
    void finishReplace();
    void scheduleReplaceCommit(uint32_t waitMs);
    void drainQueue();

    bool safeToReplace(const std::string &deleted) const;
    bool hasSurrounding() const;
    bool isDisabled() const;
    uint32_t replaceWaitMs(int deleteChars) const;

    void updateSurroundingAfterDelete(int deleteChars);
    void updateSurroundingAfterCommit(const std::string &insert);
    void clearPreedit();
    void forwardQueuedKey(const Key &key);

    EventLoop &eventLoop_;
    InputContext *ic_;
    ConfigPtr config_;
    VfkSession *session_ = nullptr;

    uint64_t configGeneration_ = 0;
    std::string program_;
    uint32_t fieldKind_ = VFK_FIELD_TEXT;
    uint32_t mode_ = VFK_MODE_PREEDIT;
    bool disabled_ = false;

    bool busy_ = false;
    bool usingClientPreedit_ = false;
    bool hasPreedit_ = false;
    std::deque<QueuedKey> queue_;

    std::unique_ptr<EventSourceTime> replaceTimer_;
    std::string pendingText_;
    bool pendingPassKey_ = false;
    std::optional<Key> pendingRawKey_;
};

} // namespace fcitx

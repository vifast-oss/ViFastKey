// Test adapter Fcitx5 không cần GUI/X server.
//
// Dùng InputContext giả lập để kiểm tra các bất biến quan trọng:
// preedit/commit, reset không mất chữ, focus-out không commit hai lần,
// password pass-through, surrounding guard và transaction Replace.

#include <fcitx-utils/event.h>
#include <fcitx-utils/keysym.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <fcitx/text.h>

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <memory>
#include <string>
#include <vector>

#include <unistd.h>

#include "config_store.h"
#include "vifastkey_state.h"

namespace {

int failures = 0;

#define CHECK(condition)                                                       \
    do {                                                                       \
        if (!(condition)) {                                                    \
            std::cerr << __FILE__ << ':' << __LINE__                           \
                      << " CHECK failed: " #condition << '\n';                 \
            ++failures;                                                        \
        }                                                                      \
    } while (0)

class TestInputContext final : public fcitx::InputContext {
public:
    explicit TestInputContext(fcitx::InputContextManager &manager,
                              const char *program = "test")
        : fcitx::InputContext(manager, program) {}
    ~TestInputContext() override { destroy(); }

    const char *frontend() const override { return "test"; }

    void commitStringImpl(const std::string &text) override {
        commits.push_back(text);
    }
    void deleteSurroundingTextImpl(int offset, unsigned int size) override {
        deletes.emplace_back(offset, size);
    }
    void forwardKeyImpl(const fcitx::ForwardKeyEvent &event) override {
        forwards.push_back(event.rawKey());
    }
    void updatePreeditImpl() override {}

    struct DeleteCall {
        int offset;
        unsigned int size;
    };
    std::vector<std::string> commits;
    std::vector<DeleteCall> deletes;
    std::vector<fcitx::Key> forwards;
};

std::string testConfigPath(const std::string &content = "{}") {
    static int counter = 0;
    const auto path = std::filesystem::temp_directory_path() /
                      ("vifastkey-fcitx5-test-" +
                       std::to_string(static_cast<long long>(getpid())) + "-" +
                       std::to_string(counter++) + ".json");
    std::ofstream out(path, std::ios::binary);
    out << content;
    return path.string();
}

std::shared_ptr<fcitx::ConfigStore> makeConfig(const std::string &content = "{}") {
    return std::make_shared<fcitx::ConfigStore>(testConfigPath(content));
}

void runLoop(fcitx::EventLoop &loop, int milliseconds = 80) {
    const uint64_t deadline =
        fcitx::now(CLOCK_MONOTONIC) +
        static_cast<uint64_t>(milliseconds) * 1000ULL;
    auto stop = loop.addTimeEvent(
        CLOCK_MONOTONIC, deadline, 0,
        [&loop](fcitx::EventSourceTime *, uint64_t) {
            loop.exit();
            return false;
        });
    if (stop) {
        stop->setOneShot();
    }
    loop.exec();
}

fcitx::KeyEvent keyEvent(TestInputContext &context, fcitx::KeySym sym,
                         fcitx::KeyStates states = fcitx::KeyStates()) {
    return fcitx::KeyEvent(&context, fcitx::Key(sym, states));
}

void typeAscii(TestInputContext &context, fcitx::VifastkeyState &state,
               const std::string &text) {
    for (char ch : text) {
        auto event =
            keyEvent(context, static_cast<fcitx::KeySym>(static_cast<unsigned char>(ch)));
        state.keyEvent(event);
        CHECK(event.filtered());
    }
}

void testPreeditAndCommit() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::Preedit);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    typeAscii(context, state, "tieengs");
    CHECK(context.inputPanel().clientPreedit().toString() == "tiếng");

    auto space = keyEvent(context, FcitxKey_space);
    state.keyEvent(space);
    CHECK(space.filtered());
    CHECK(context.commits.size() == 1);
    if (context.commits.size() == 1) {
        CHECK(context.commits.front() == "tiếng ");
    }
    CHECK(context.inputPanel().clientPreedit().empty());
}

void testResetCommitsComposition() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::Preedit);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    typeAscii(context, state, "vieejt");
    CHECK(context.inputPanel().clientPreedit().toString() == "việt");

    state.reset();
    CHECK(context.commits.size() == 1);
    if (context.commits.size() == 1) {
        CHECK(context.commits.front() == "việt");
    }
    CHECK(context.inputPanel().clientPreedit().empty());
}

void testFocusOutDoesNotDoubleCommitClientPreedit() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::Preedit);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    typeAscii(context, state, "vieejt");
    CHECK(context.inputPanel().clientPreedit().toString() == "việt");

    fcitx::FocusOutEvent event(&context);
    state.deactivate(event);

    // Fcitx core? Trong test core không có; client preedit do frontend commit.
    // Adapter phải không commit thêm để tránh lặp từ.
    CHECK(context.commits.empty());
    CHECK(context.inputPanel().clientPreedit().empty());
}

void testPasswordPassThrough() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlags{fcitx::CapabilityFlag::Password,
                                        fcitx::CapabilityFlag::PasswordOrSensitive});

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    auto event = keyEvent(context, FcitxKey_a);
    state.keyEvent(event);
    CHECK(!event.filtered());
    CHECK(context.commits.empty());
    CHECK(context.inputPanel().clientPreedit().empty());
}

void testInvalidConfigFallsBackToDefaults() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::Preedit);

    auto config = makeConfig("{not-json");
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    typeAscii(context, state, "tieengs");
    CHECK(context.inputPanel().clientPreedit().toString() == "tiếng");
}

void testReplaceUsesSurroundingAndCommits() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlags{fcitx::CapabilityFlag::Preedit,
                                        fcitx::CapabilityFlag::SurroundingText});
    context.surroundingText().setText("tiếng", 5, 5);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    VfkAction action = {};
    action.kind = VFK_REPLACE;
    action.delete_utf16 = 5; // "tiếng"
    action.text = "tiếng";
    state.applyAction(action, nullptr, "tiếng");

    CHECK(context.deletes.size() == 1);
    if (context.deletes.size() == 1) {
        CHECK(context.deletes.front().offset == -5);
        CHECK(context.deletes.front().size == 5);
    }
    CHECK(state.busy());

    runLoop(loop);
    CHECK(!state.busy());
    CHECK(context.commits.size() == 1);
    if (context.commits.size() == 1) {
        CHECK(context.commits.front() == "tiếng");
    }
}

void testEditPreviousWordUsesRefreshedSurrounding() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlags{
        fcitx::CapabilityFlag::Preedit,
        fcitx::CapabilityFlag::SurroundingText});
    context.surroundingText().setText("tiếng", 5, 5);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    // Không gõ lại "tiếng": engine phải adopt từ đang có trước caret rồi
    // đổi tone f -> huyền, tạo Replace "tiếng" -> "tiềng".
    auto event = keyEvent(context, FcitxKey_f);
    state.keyEvent(event);
    CHECK(event.filtered());
    CHECK(context.deletes.size() == 1);
    if (context.deletes.size() == 1) {
        CHECK(context.deletes.front().offset == -3);
        CHECK(context.deletes.front().size == 3);
    }
    CHECK(state.busy());

    runLoop(loop);
    CHECK(!state.busy());
    CHECK(context.commits.size() == 1);
    if (context.commits.size() == 1) {
        CHECK(context.commits.front() == "ềng");
    }
}

void testReplacePassKeyIsForwardedAfterCommit() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::SurroundingText);
    context.surroundingText().setText("tiếng", 5, 5);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    VfkAction action = {};
    action.kind = VFK_REPLACE;
    action.delete_utf16 = 3; // "ếng"
    action.text = "ềng";
    action.pass_key = true;
    state.applyAction(action, nullptr, "tiếng", fcitx::Key(FcitxKey_space));

    CHECK(context.deletes.size() == 1);
    CHECK(state.busy());
    runLoop(loop);
    CHECK(!state.busy());
    CHECK(context.commits.size() == 1);
    if (context.commits.size() == 1) {
        CHECK(context.commits.front() == "ềng");
    }
    // Phím gốc (space) phải được forward sau commit, không bị nuốt.
    CHECK(context.forwards.size() == 2);
}

void testAppProfileReplaceMode() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlags{
        fcitx::CapabilityFlag::Preedit,
        fcitx::CapabilityFlag::SurroundingText});
    context.surroundingText().setText("", 0, 0);

    auto config = makeConfig(
        R"({"app_profiles":{"test":{"output_mode":"replace","edit_previous_word":false}}})");
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    // Plain letter đi thẳng xuống app (PassThrough), không tạo preedit.
    auto a = keyEvent(context, FcitxKey_a);
    state.keyEvent(a);
    CHECK(!a.filtered());

    // Giả lập app đã nhận "a"; phím s tạo "á" bằng Replace.
    context.surroundingText().setText("a", 1, 1);
    auto s = keyEvent(context, FcitxKey_s);
    state.keyEvent(s);
    CHECK(s.filtered());
    CHECK(context.deletes.size() == 1);
    if (context.deletes.size() == 1) {
        CHECK(context.deletes.front().offset == -1);
        CHECK(context.deletes.front().size == 1);
    }

    runLoop(loop);
    CHECK(context.commits.size() == 1);
    if (context.commits.size() == 1) {
        CHECK(context.commits.front() == "á");
    }
}

void testReplaceRejectsStaleSurrounding() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::SurroundingText);
    context.surroundingText().setText("hello", 5, 5);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    VfkAction action = {};
    action.kind = VFK_REPLACE;
    action.delete_utf16 = 5; // "tiếng" không khớp "hello"
    action.text = "tiếng";
    state.applyAction(action, nullptr, "tiếng");

    CHECK(context.deletes.empty());
    CHECK(context.commits.empty());
    CHECK(!state.busy());
}

void testReplaceRejectsSelection() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::SurroundingText);
    context.surroundingText().setText("tiếng", 5, 2);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    VfkAction action = {};
    action.kind = VFK_REPLACE;
    action.delete_utf16 = 3; // "ếng"
    action.text = "ếng";
    state.applyAction(action, nullptr, "tiếng");

    CHECK(context.deletes.empty());
    CHECK(context.commits.empty());
}

void testReplaceCountsCodepointsForAstralCharacters() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlag::SurroundingText);
    // Emoji chiếm 2 UTF-16 code unit nhưng 1 code point.
    context.surroundingText().setText("🙂a", 2, 2);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    VfkAction action = {};
    action.kind = VFK_REPLACE;
    action.delete_utf16 = 3; // 🙂 (2) + a (1)
    action.text = "X";
    state.applyAction(action, nullptr, "🙂a");

    CHECK(context.deletes.size() == 1);
    if (context.deletes.size() == 1) {
        CHECK(context.deletes.front().offset == -2);
        CHECK(context.deletes.front().size == 2);
    }

    runLoop(loop);
    CHECK(context.commits.size() == 1);
    if (context.commits.size() == 1) {
        CHECK(context.commits.front() == "X");
    }
}

void testQueuedKeyIsForwardedWhenReplaceIsBusy() {
    fcitx::EventLoop loop;
    fcitx::InputContextManager manager;
    TestInputContext context(manager);
    context.setCapabilityFlags(fcitx::CapabilityFlags{fcitx::CapabilityFlag::Preedit,
                                        fcitx::CapabilityFlag::SurroundingText});
    context.surroundingText().setText("tiếng", 5, 5);

    auto config = makeConfig();
    fcitx::VifastkeyState state(loop, &context, config);
    state.activate("test");

    VfkAction action = {};
    action.kind = VFK_REPLACE;
    action.delete_utf16 = 5;
    action.text = "tiếng";
    state.applyAction(action, nullptr, "tiếng");
    CHECK(state.busy());

    // Engine đang rỗng: Backspace trả PassThrough. Key bị queue trong lúc
    // Replace chạy phải được forward sau khi commit xong, không bị nuốt.
    auto queued = keyEvent(context, FcitxKey_BackSpace);
    state.keyEvent(queued);
    CHECK(queued.filtered());

    runLoop(loop);
    CHECK(!state.busy());
    CHECK(context.commits.size() == 1);
    CHECK(context.forwards.size() == 2); // press + release của Backspace
}

} // namespace

int main() {
    testPreeditAndCommit();
    testResetCommitsComposition();
    testFocusOutDoesNotDoubleCommitClientPreedit();
    testPasswordPassThrough();
    testInvalidConfigFallsBackToDefaults();
    testReplaceUsesSurroundingAndCommits();
    testEditPreviousWordUsesRefreshedSurrounding();
    testReplacePassKeyIsForwardedAfterCommit();
    testAppProfileReplaceMode();
    testReplaceRejectsStaleSurrounding();
    testReplaceRejectsSelection();
    testReplaceCountsCodepointsForAstralCharacters();
    testQueuedKeyIsForwardedWhenReplaceIsBusy();

    if (failures != 0) {
        std::cerr << failures << " test(s) failed\n";
        return 1;
    }
    std::cout << "vifastkey-fcitx5 tests passed\n";
    return 0;
}

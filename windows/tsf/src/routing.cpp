#include "routing.h"

#include <windows.h>

#include <cstdint>

namespace {

constexpr wchar_t kMappingName[] = L"Local\\ViFastKeyTsfState";
constexpr uint32_t kMagic = 0x56464B54;  // 'VFKT'
constexpr uint32_t kVersion = 1;

struct SharedState {
    uint32_t magic;
    uint32_t version;
    uint32_t pid;
    uint32_t active;
    uint64_t window;
};

HANDLE g_mapping = nullptr;
SharedState* g_view = nullptr;

bool EnsureMapping() {
    if (g_view != nullptr) {
        return true;
    }
    HANDLE mapping = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, 0,
                                        sizeof(SharedState), kMappingName);
    if (mapping == nullptr) {
        mapping = OpenFileMappingW(FILE_MAP_READ | FILE_MAP_WRITE, FALSE, kMappingName);
    }
    if (mapping == nullptr) {
        return false;
    }
    void* view = MapViewOfFile(mapping, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, sizeof(SharedState));
    if (view == nullptr) {
        CloseHandle(mapping);
        return false;
    }
    g_mapping = mapping;
    g_view = static_cast<SharedState*>(view);
    if (g_view->magic != kMagic || g_view->version != kVersion) {
        g_view->magic = kMagic;
        g_view->version = kVersion;
        g_view->pid = 0;
        g_view->active = 0;
        g_view->window = 0;
    }
    return true;
}

}  // namespace

void PublishTsfActive() {
    if (!EnsureMapping()) {
        return;
    }
    InterlockedExchange(reinterpret_cast<volatile LONG*>(&g_view->pid),
                        static_cast<LONG>(GetCurrentProcessId()));
    g_view->window = static_cast<uint64_t>(reinterpret_cast<uintptr_t>(GetForegroundWindow()));
    InterlockedExchange(reinterpret_cast<volatile LONG*>(&g_view->active), 1);
}

void ClearTsfActive() {
    if (g_view == nullptr) {
        return;
    }
    if (static_cast<uint32_t>(g_view->pid) == GetCurrentProcessId()) {
        InterlockedExchange(reinterpret_cast<volatile LONG*>(&g_view->active), 0);
        InterlockedExchange(reinterpret_cast<volatile LONG*>(&g_view->pid), 0);
        g_view->window = 0;
    }
}

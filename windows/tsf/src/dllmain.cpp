#include "globals.h"

HINSTANCE g_hInstance = nullptr;
LONG g_cDllRef = 0;

BOOL APIENTRY DllMain(HMODULE module, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        g_hInstance = module;
        DisableThreadLibraryCalls(module);
    }
    return TRUE;
}

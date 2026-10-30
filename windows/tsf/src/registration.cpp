#include <msctf.h>
#include <windows.h>

#include <iterator>
#include <string>

#include "globals.h"

namespace {

constexpr LANGID kVietnameseLangId = 0x042A;

struct ComInit {
    HRESULT hr = E_FAIL;
    bool uninit = false;

    ComInit() {
        hr = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
        uninit = SUCCEEDED(hr);
        if (hr == RPC_E_CHANGED_MODE) {
            hr = S_OK;
        }
    }

    ~ComInit() {
        if (uninit) {
            CoUninitialize();
        }
    }
};

std::wstring ClsidKey() {
    wchar_t guid[64] = {};
    StringFromGUID2(CLSID_ViFastKey, guid, static_cast<int>(std::size(guid)));
    return std::wstring(L"Software\\Classes\\CLSID\\") + guid;
}

HRESULT SetStringValue(HKEY key, const wchar_t* name, const std::wstring& value) {
    return RegSetValueExW(key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(value.c_str()),
                          static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
}

HRESULT RegisterComServer(const std::wstring& module_path) {
    const std::wstring clsid_key = ClsidKey();
    HKEY key = nullptr;
    LRESULT status = RegCreateKeyExW(HKEY_CURRENT_USER, clsid_key.c_str(), 0, nullptr, 0,
                                     KEY_WRITE, nullptr, &key, nullptr);
    if (status != ERROR_SUCCESS) {
        return HRESULT_FROM_WIN32(status);
    }
    SetStringValue(key, nullptr, L"ViFastKey TSF");
    RegCloseKey(key);

    status = RegCreateKeyExW(HKEY_CURRENT_USER, (clsid_key + L"\\InprocServer32").c_str(), 0,
                             nullptr, 0, KEY_WRITE, nullptr, &key, nullptr);
    if (status != ERROR_SUCCESS) {
        return HRESULT_FROM_WIN32(status);
    }
    SetStringValue(key, nullptr, module_path);
    SetStringValue(key, L"ThreadingModel", L"Apartment");
    RegCloseKey(key);
    return S_OK;
}

HRESULT RegisterProfile() {
    ITfInputProcessorProfiles* profiles = nullptr;
    HRESULT hr = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_ITfInputProcessorProfiles,
                                  reinterpret_cast<void**>(&profiles));
    if (FAILED(hr)) {
        return hr;
    }

    profiles->RemoveLanguageProfile(CLSID_ViFastKey, kVietnameseLangId, GUID_ViFastKeyProfile);
    profiles->Register(CLSID_ViFastKey);
    hr = profiles->AddLanguageProfile(CLSID_ViFastKey, kVietnameseLangId, GUID_ViFastKeyProfile,
                                      L"ViFastKey", static_cast<ULONG>(-1), nullptr, 0, 0);
    profiles->Release();
    if (FAILED(hr)) {
        return hr;
    }

    ITfCategoryMgr* categories = nullptr;
    hr = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr,
                          reinterpret_cast<void**>(&categories));
    if (FAILED(hr)) {
        return hr;
    }
    categories->RegisterCategory(CLSID_ViFastKey, GUID_TFCAT_TIP_KEYBOARD, CLSID_ViFastKey);
    categories->RegisterCategory(CLSID_ViFastKey, GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
                                 CLSID_ViFastKey);
    categories->RegisterCategory(CLSID_ViFastKey, GUID_TFCAT_TIPCAP_SECUREMODE, CLSID_ViFastKey);
    categories->RegisterCategory(CLSID_ViFastKey, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
                                 CLSID_ViFastKey);
    categories->Release();
    return S_OK;
}

void UnregisterProfile() {
    ITfInputProcessorProfiles* profiles = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                   IID_ITfInputProcessorProfiles,
                                   reinterpret_cast<void**>(&profiles)))) {
        profiles->RemoveLanguageProfile(CLSID_ViFastKey, kVietnameseLangId, GUID_ViFastKeyProfile);
        profiles->Unregister(CLSID_ViFastKey);
        profiles->Release();
    }

    ITfCategoryMgr* categories = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                                   IID_ITfCategoryMgr, reinterpret_cast<void**>(&categories)))) {
        categories->UnregisterCategory(CLSID_ViFastKey, GUID_TFCAT_TIP_KEYBOARD, CLSID_ViFastKey);
        categories->UnregisterCategory(CLSID_ViFastKey, GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
                                      CLSID_ViFastKey);
        categories->UnregisterCategory(CLSID_ViFastKey, GUID_TFCAT_TIPCAP_SECUREMODE,
                                      CLSID_ViFastKey);
        categories->UnregisterCategory(CLSID_ViFastKey, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
                                      CLSID_ViFastKey);
        categories->Release();
    }
}

}  // namespace

extern "C" HRESULT STDAPICALLTYPE DllRegisterServer() {
    ComInit com;
    if (FAILED(com.hr)) {
        return com.hr;
    }

    wchar_t module_path[MAX_PATH] = {};
    DWORD length = GetModuleFileNameW(g_hInstance, module_path, static_cast<DWORD>(std::size(module_path)));
    if (length == 0 || static_cast<size_t>(length) >= std::size(module_path)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }

    HRESULT hr = RegisterComServer(module_path);
    if (FAILED(hr)) {
        return hr;
    }
    return RegisterProfile();
}

extern "C" HRESULT STDAPICALLTYPE DllUnregisterServer() {
    ComInit com;
    if (FAILED(com.hr)) {
        return com.hr;
    }

    UnregisterProfile();
    RegDeleteTreeW(HKEY_CURRENT_USER, ClsidKey().c_str());
    return S_OK;
}

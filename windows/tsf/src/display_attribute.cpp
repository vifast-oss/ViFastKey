#include "display_attribute.h"

#include <new>
#include <oleauto.h>

#include "globals.h"

namespace {

constexpr GUID kDisplayAttributeGuid = {
    0xc2b5daf3, 0x7a4e, 0x4b8c, {0x9d, 0x6f, 0x0e, 0x3a, 0x2b, 0x1c, 0x4d, 0x13}};

}  // namespace

const GUID GUID_ViFastKeyDisplayAttribute = kDisplayAttributeGuid;

DisplayAttributeInfo::DisplayAttributeInfo() {
    InterlockedIncrement(&g_cDllRef);
}

DisplayAttributeInfo::~DisplayAttributeInfo() {
    InterlockedDecrement(&g_cDllRef);
}

EnumDisplayAttributeInfo::EnumDisplayAttributeInfo(ULONG index) : index_(index) {
    InterlockedIncrement(&g_cDllRef);
}

EnumDisplayAttributeInfo::~EnumDisplayAttributeInfo() {
    InterlockedDecrement(&g_cDllRef);
}

STDMETHODIMP DisplayAttributeInfo::QueryInterface(REFIID riid, void** ppv) {
    if (ppv == nullptr) {
        return E_POINTER;
    }
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfDisplayAttributeInfo)) {
        *ppv = static_cast<ITfDisplayAttributeInfo*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

STDMETHODIMP_(ULONG) DisplayAttributeInfo::AddRef() {
    return static_cast<ULONG>(InterlockedIncrement(&ref_));
}

STDMETHODIMP_(ULONG) DisplayAttributeInfo::Release() {
    LONG value = InterlockedDecrement(&ref_);
    if (value == 0) {
        delete this;
    }
    return static_cast<ULONG>(value);
}

STDMETHODIMP DisplayAttributeInfo::GetGUID(GUID* pguid) {
    if (pguid == nullptr) {
        return E_POINTER;
    }
    *pguid = GUID_ViFastKeyDisplayAttribute;
    return S_OK;
}

STDMETHODIMP DisplayAttributeInfo::GetDescription(BSTR* pbstrDesc) {
    if (pbstrDesc == nullptr) {
        return E_POINTER;
    }
    *pbstrDesc = SysAllocString(L"ViFastKey composition");
    return *pbstrDesc == nullptr ? E_OUTOFMEMORY : S_OK;
}

STDMETHODIMP DisplayAttributeInfo::GetAttributeInfo(TF_DISPLAYATTRIBUTE* pda) {
    if (pda == nullptr) {
        return E_POINTER;
    }
    *pda = {};
    pda->crText.type = TF_CT_NONE;
    pda->crBk.type = TF_CT_NONE;
    pda->lsStyle = TF_LS_SOLID;
    pda->fBoldLine = FALSE;
    pda->crLine.type = TF_CT_COLORREF;
    pda->crLine.cr = RGB(0, 102, 204);
    pda->bAttr = TF_ATTR_INPUT;
    return S_OK;
}

STDMETHODIMP DisplayAttributeInfo::SetAttributeInfo(const TF_DISPLAYATTRIBUTE*) {
    return E_NOTIMPL;
}

STDMETHODIMP DisplayAttributeInfo::Reset() {
    return S_OK;
}

STDMETHODIMP EnumDisplayAttributeInfo::QueryInterface(REFIID riid, void** ppv) {
    if (ppv == nullptr) {
        return E_POINTER;
    }
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IEnumTfDisplayAttributeInfo)) {
        *ppv = static_cast<IEnumTfDisplayAttributeInfo*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

STDMETHODIMP_(ULONG) EnumDisplayAttributeInfo::AddRef() {
    return static_cast<ULONG>(InterlockedIncrement(&ref_));
}

STDMETHODIMP_(ULONG) EnumDisplayAttributeInfo::Release() {
    LONG value = InterlockedDecrement(&ref_);
    if (value == 0) {
        delete this;
    }
    return static_cast<ULONG>(value);
}

STDMETHODIMP EnumDisplayAttributeInfo::Clone(IEnumTfDisplayAttributeInfo** ppEnum) {
    if (ppEnum == nullptr) {
        return E_POINTER;
    }
    *ppEnum = new (std::nothrow) EnumDisplayAttributeInfo(index_);
    return *ppEnum == nullptr ? E_OUTOFMEMORY : S_OK;
}

STDMETHODIMP EnumDisplayAttributeInfo::Next(ULONG ulCount, ITfDisplayAttributeInfo** rgInfo,
                                            ULONG* pcFetched) {
    if (rgInfo == nullptr) {
        return E_POINTER;
    }
    if (pcFetched != nullptr) {
        *pcFetched = 0;
    }
    if (ulCount == 0 || index_ > 0) {
        return S_OK;
    }
    rgInfo[0] = new (std::nothrow) DisplayAttributeInfo();
    if (rgInfo[0] == nullptr) {
        return E_OUTOFMEMORY;
    }
    ++index_;
    if (pcFetched != nullptr) {
        *pcFetched = 1;
    }
    return S_OK;
}

STDMETHODIMP EnumDisplayAttributeInfo::Reset() {
    index_ = 0;
    return S_OK;
}

STDMETHODIMP EnumDisplayAttributeInfo::Skip(ULONG ulCount) {
    index_ += ulCount;
    return S_OK;
}

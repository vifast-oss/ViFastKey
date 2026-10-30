#include "globals.h"

#include <new>

#include "text_service.h"

namespace {

class ClassFactory final : public IClassFactory {
  public:
    ClassFactory() {
        InterlockedIncrement(&g_cDllRef);
    }

    ~ClassFactory() {
        InterlockedDecrement(&g_cDllRef);
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (ppv == nullptr) {
            return E_POINTER;
        }
        *ppv = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IClassFactory)) {
            *ppv = static_cast<IClassFactory*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    STDMETHODIMP_(ULONG) AddRef() override {
        return static_cast<ULONG>(InterlockedIncrement(&ref_));
    }

    STDMETHODIMP_(ULONG) Release() override {
        LONG value = InterlockedDecrement(&ref_);
        if (value == 0) {
            delete this;
        }
        return static_cast<ULONG>(value);
    }

    STDMETHODIMP CreateInstance(IUnknown* outer, REFIID riid, void** ppv) override {
        if (ppv == nullptr) {
            return E_POINTER;
        }
        *ppv = nullptr;
        if (outer != nullptr) {
            return CLASS_E_NOAGGREGATION;
        }
        auto* service = new (std::nothrow) TextService();
        if (service == nullptr) {
            return E_OUTOFMEMORY;
        }
        HRESULT hr = service->QueryInterface(riid, ppv);
        service->Release();
        return hr;
    }

    STDMETHODIMP LockServer(BOOL lock) override {
        if (lock) {
            InterlockedIncrement(&g_cDllRef);
        } else {
            InterlockedDecrement(&g_cDllRef);
        }
        return S_OK;
    }

  private:
    LONG ref_ = 1;
};

}  // namespace

extern "C" HRESULT STDAPICALLTYPE DllGetClassObject(REFCLSID rclsid, REFIID riid, void** ppv) {
    if (ppv == nullptr) {
        return E_POINTER;
    }
    *ppv = nullptr;
    if (!IsEqualCLSID(rclsid, CLSID_ViFastKey)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    auto* factory = new (std::nothrow) ClassFactory();
    if (factory == nullptr) {
        return E_OUTOFMEMORY;
    }
    HRESULT hr = factory->QueryInterface(riid, ppv);
    factory->Release();
    return hr;
}

extern "C" HRESULT STDAPICALLTYPE DllCanUnloadNow() {
    return g_cDllRef == 0 ? S_OK : S_FALSE;
}

#pragma once

#include <msctf.h>
#include <windows.h>

extern const GUID GUID_ViFastKeyDisplayAttribute;

class DisplayAttributeInfo final : public ITfDisplayAttributeInfo {
  public:
    DisplayAttributeInfo();

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    STDMETHODIMP GetGUID(GUID* pguid) override;
    STDMETHODIMP GetDescription(BSTR* pbstrDesc) override;
    STDMETHODIMP GetAttributeInfo(TF_DISPLAYATTRIBUTE* pda) override;
    STDMETHODIMP SetAttributeInfo(const TF_DISPLAYATTRIBUTE* pda) override;
    STDMETHODIMP Reset() override;

  private:
    ~DisplayAttributeInfo();
    LONG ref_ = 1;
};

class EnumDisplayAttributeInfo final : public IEnumTfDisplayAttributeInfo {
  public:
    explicit EnumDisplayAttributeInfo(ULONG index = 0);

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    STDMETHODIMP Clone(IEnumTfDisplayAttributeInfo** ppEnum) override;
    STDMETHODIMP Next(ULONG ulCount, ITfDisplayAttributeInfo** rgInfo,
                      ULONG* pcFetched) override;
    STDMETHODIMP Reset() override;
    STDMETHODIMP Skip(ULONG ulCount) override;

  private:
    ~EnumDisplayAttributeInfo();
    LONG ref_ = 1;
    ULONG index_ = 0;
};

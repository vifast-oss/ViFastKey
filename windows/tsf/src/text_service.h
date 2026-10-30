#pragma once

#include <msctf.h>
#include <windows.h>

#include <cstdint>
#include <string>

#include "engine.h"

struct SurroundingText {
    std::string utf8;
    uint32_t utf16_len = 0;
};

class ActionEditSession;
class FieldEditSession;
class KeyEditSession;

class TextService final : public ITfTextInputProcessorEx,
                          public ITfThreadMgrEventSink,
                          public ITfKeyEventSink,
                          public ITfCompositionSink,
                          public ITfDisplayAttributeProvider {
  public:
    TextService();

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // ITfTextInputProcessor / ITfTextInputProcessorEx
    STDMETHODIMP Activate(ITfThreadMgr* ptim, TfClientId tid) override;
    STDMETHODIMP ActivateEx(ITfThreadMgr* ptim, TfClientId tid, DWORD dwFlags) override;
    STDMETHODIMP Deactivate() override;

    // ITfThreadMgrEventSink
    STDMETHODIMP OnInitDocumentMgr(ITfDocumentMgr* pdim) override;
    STDMETHODIMP OnUninitDocumentMgr(ITfDocumentMgr* pdim) override;
    STDMETHODIMP OnSetFocus(ITfDocumentMgr* pdimFocus, ITfDocumentMgr* pdimPrevFocus) override;
    STDMETHODIMP OnPushContext(ITfContext* pic) override;
    STDMETHODIMP OnPopContext(ITfContext* pic) override;

    // ITfKeyEventSink
    STDMETHODIMP OnSetFocus(BOOL fForeground) override;
    STDMETHODIMP OnTestKeyDown(ITfContext* pic, WPARAM wParam, LPARAM lParam,
                               BOOL* pfEaten) override;
    STDMETHODIMP OnKeyDown(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) override;
    STDMETHODIMP OnTestKeyUp(ITfContext* pic, WPARAM wParam, LPARAM lParam,
                             BOOL* pfEaten) override;
    STDMETHODIMP OnKeyUp(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) override;
    STDMETHODIMP OnPreservedKey(ITfContext* pic, REFGUID rguid, BOOL* pfEaten) override;

    // ITfCompositionSink
    STDMETHODIMP OnCompositionTerminated(TfEditCookie ec, ITfComposition* pComposition) override;

    // ITfDisplayAttributeProvider
    STDMETHODIMP EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo** ppEnum) override;
    STDMETHODIMP GetDisplayAttributeInfo(REFGUID guid,
                                         ITfDisplayAttributeInfo** ppInfo) override;

  private:
    friend class ActionEditSession;
    friend class FieldEditSession;
    friend class KeyEditSession;

    ~TextService();

    HRESULT AdviseSinks();
    void UnadviseSinks();
    void ReleaseContext();
    bool AttachContext(ITfDocumentMgr* pdimFocus);
    bool EnsureContext();
    void PublishRouting();
    void ClearRouting();
    bool BackendAllowsTsf() const;
    void ReloadConfigIfNeeded();

    HRESULT ApplyActionOn(ITfContext* context, const ActionData& action);
    HRESULT FlushCurrentContext();
    HRESULT ExecuteToggle();
    HRESULT UpdateAppAndField();

    SurroundingText GetSurrounding(ITfContext* context, TfEditCookie ec) const;
    uint32_t ReadFieldKind(ITfContext* context, TfEditCookie ec) const;
    std::string AppIdFromForeground() const;

    HRESULT HandleAction(ITfContext* context, TfEditCookie ec, const ActionData& action,
                         bool* eaten);
    HRESULT HandleKey(ITfContext* context, TfEditCookie ec, const std::string& key, bool ctrl,
                      bool alt, bool meta, bool* eaten);
    HRESULT ApplyPlan(ITfContext* context, TfEditCookie ec, const PlanData& plan, bool* eaten);

    HRESULT SetPreedit(ITfContext* context, TfEditCookie ec, const std::wstring& text,
                       uint32_t cursor_utf16);
    HRESULT CommitText(ITfContext* context, TfEditCookie ec, const std::wstring& text);
    HRESULT EndComposition(TfEditCookie ec);
    HRESULT ReplaceBeforeCaret(ITfContext* context, TfEditCookie ec, uint32_t delete_utf16,
                               const std::wstring& text);
    HRESULT InsertAtCaret(ITfContext* context, TfEditCookie ec, const std::wstring& text);
    void SetCaret(ITfContext* context, TfEditCookie ec, ITfRange* range, uint32_t cursor_utf16);
    void ApplyDisplayAttribute(ITfContext* context, TfEditCookie ec, ITfRange* range);

    LONG ref_ = 1;
    ITfThreadMgr* thread_mgr_ = nullptr;
    TfClientId client_id_ = TF_CLIENTID_NULL;
    ITfDocumentMgr* doc_mgr_ = nullptr;
    ITfContext* context_ = nullptr;
    ITfComposition* composition_ = nullptr;
    DWORD thread_sink_cookie_ = 0;
    DWORD key_sink_cookie_ = 0;

    Engine engine_;
    std::string backend_ = "tsf";
    std::string loaded_config_;
    bool ending_composition_ = false;
};

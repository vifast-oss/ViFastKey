#include "text_service.h"

#include <inputscope.h>
#include <msctf.h>
#include <oleauto.h>

#include <climits>
#include <cstring>
#include <cwctype>
#include <iterator>
#include <new>
#include <utility>

#include "config.h"
#include "display_attribute.h"
#include "globals.h"
#include "keymap.h"
#include "routing.h"
#include "utf.h"

namespace {

constexpr LONG kMaxSurrounding = 128;

// GUID_PROP_INPUTSCOPE từ inputscope.h. Khai báo tại đây để DLL không phụ thuộc
// inputscope.lib/uuid.lib của từng bản Windows SDK.
constexpr GUID kGuidPropInputScope = {
    0x1713dd5a, 0x68e7, 0x4a5b, {0x9a, 0xf6, 0x59, 0x2a, 0x59, 0x5c, 0x77, 0x8d}};

bool IsControlDown() {
    return (GetKeyState(VK_CONTROL) & 0x8000) != 0;
}

bool IsShiftDown() {
    return (GetKeyState(VK_SHIFT) & 0x8000) != 0;
}

bool IsAltDown() {
    return (GetKeyState(VK_MENU) & 0x8000) != 0;
}

bool IsMetaDown() {
    return (GetKeyState(VK_LWIN) & 0x8000) != 0 || (GetKeyState(VK_RWIN) & 0x8000) != 0;
}

bool IsTerminalApp(const std::string& app) {
    return app == "windowsterminal" || app == "cmd" || app == "conhost" ||
           app == "powershell" || app == "pwsh" || app == "wt";
}

}  // namespace

class ActionEditSession final : public ITfEditSession {
  public:
    ActionEditSession(TextService* owner, ITfContext* context, ActionData action)
        : owner_(owner), context_(context), action_(std::move(action)) {
        owner_->AddRef();
        if (context_ != nullptr) {
            context_->AddRef();
        }
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (ppv == nullptr) {
            return E_POINTER;
        }
        *ppv = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfEditSession)) {
            *ppv = static_cast<ITfEditSession*>(this);
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

    STDMETHODIMP DoEditSession(TfEditCookie ec) override {
        return owner_->HandleAction(context_, ec, action_, &eaten_);
    }

    bool eaten() const {
        return eaten_;
    }

  private:
    ~ActionEditSession() {
        if (context_ != nullptr) {
            context_->Release();
        }
        owner_->Release();
    }

    LONG ref_ = 1;
    TextService* owner_ = nullptr;
    ITfContext* context_ = nullptr;
    ActionData action_;
    bool eaten_ = false;
};

class KeyEditSession final : public ITfEditSession {
  public:
    KeyEditSession(TextService* owner, ITfContext* context, std::string key, bool ctrl, bool alt,
                   bool meta)
        : owner_(owner),
          context_(context),
          key_(std::move(key)),
          ctrl_(ctrl),
          alt_(alt),
          meta_(meta) {
        owner_->AddRef();
        if (context_ != nullptr) {
            context_->AddRef();
        }
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (ppv == nullptr) {
            return E_POINTER;
        }
        *ppv = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfEditSession)) {
            *ppv = static_cast<ITfEditSession*>(this);
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

    STDMETHODIMP DoEditSession(TfEditCookie ec) override {
        return owner_->HandleKey(context_, ec, key_, ctrl_, alt_, meta_, &eaten_);
    }

    bool eaten() const {
        return eaten_;
    }

  private:
    ~KeyEditSession() {
        if (context_ != nullptr) {
            context_->Release();
        }
        owner_->Release();
    }

    LONG ref_ = 1;
    TextService* owner_ = nullptr;
    ITfContext* context_ = nullptr;
    std::string key_;
    bool ctrl_ = false;
    bool alt_ = false;
    bool meta_ = false;
    bool eaten_ = false;
};

class FieldEditSession final : public ITfEditSession {
  public:
    FieldEditSession(TextService* owner, ITfContext* context) : owner_(owner), context_(context) {
        owner_->AddRef();
        if (context_ != nullptr) {
            context_->AddRef();
        }
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv) override {
        if (ppv == nullptr) {
            return E_POINTER;
        }
        *ppv = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfEditSession)) {
            *ppv = static_cast<ITfEditSession*>(this);
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

    STDMETHODIMP DoEditSession(TfEditCookie ec) override {
        field_ = owner_->ReadFieldKind(context_, ec);
        return S_OK;
    }

    uint32_t field() const {
        return field_;
    }

  private:
    ~FieldEditSession() {
        if (context_ != nullptr) {
            context_->Release();
        }
        owner_->Release();
    }

    LONG ref_ = 1;
    TextService* owner_ = nullptr;
    ITfContext* context_ = nullptr;
    uint32_t field_ = VFK_FIELD_TEXT;
};

TextService::TextService() {
    InterlockedIncrement(&g_cDllRef);
}

TextService::~TextService() {
    if (thread_mgr_ != nullptr) {
        UnadviseSinks();
    }
    ReleaseContext();
    engine_.Stop();
    if (thread_mgr_ != nullptr) {
        thread_mgr_->Release();
        thread_mgr_ = nullptr;
    }
    InterlockedDecrement(&g_cDllRef);
}

STDMETHODIMP TextService::QueryInterface(REFIID riid, void** ppv) {
    if (ppv == nullptr) {
        return E_POINTER;
    }
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfTextInputProcessor) ||
        IsEqualIID(riid, IID_ITfTextInputProcessorEx)) {
        *ppv = static_cast<ITfTextInputProcessorEx*>(this);
    } else if (IsEqualIID(riid, IID_ITfThreadMgrEventSink)) {
        *ppv = static_cast<ITfThreadMgrEventSink*>(this);
    } else if (IsEqualIID(riid, IID_ITfKeyEventSink)) {
        *ppv = static_cast<ITfKeyEventSink*>(this);
    } else if (IsEqualIID(riid, IID_ITfCompositionSink)) {
        *ppv = static_cast<ITfCompositionSink*>(this);
    } else if (IsEqualIID(riid, IID_ITfDisplayAttributeProvider)) {
        *ppv = static_cast<ITfDisplayAttributeProvider*>(this);
    } else {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) TextService::AddRef() {
    return static_cast<ULONG>(InterlockedIncrement(&ref_));
}

STDMETHODIMP_(ULONG) TextService::Release() {
    LONG value = InterlockedDecrement(&ref_);
    if (value == 0) {
        delete this;
    }
    return static_cast<ULONG>(value);
}

STDMETHODIMP TextService::Activate(ITfThreadMgr* ptim, TfClientId tid) {
    return ActivateEx(ptim, tid, 0);
}

STDMETHODIMP TextService::ActivateEx(ITfThreadMgr* ptim, TfClientId tid, DWORD) {
    if (ptim == nullptr) {
        return E_INVALIDARG;
    }
    if (thread_mgr_ != nullptr) {
        if (ptim != thread_mgr_) {
            Deactivate();
        } else {
            // TSF có thể gọi ActivateEx lại khi đổi input method mà không
            // phát Deactivate. Không trả E_UNEXPECTED; chỉ refresh context.
            ReloadConfigIfNeeded();
            EnsureContext();
            return S_OK;
        }
    }

    thread_mgr_ = ptim;
    thread_mgr_->AddRef();
    client_id_ = tid;

    AppConfig config = LoadAppConfig();
    backend_ = config.backend;
    loaded_config_ = config.engine_json;
    if (!engine_.Start(config.engine_json)) {
        thread_mgr_->Release();
        thread_mgr_ = nullptr;
        client_id_ = TF_CLIENTID_NULL;
        return E_FAIL;
    }

    HRESULT hr = AdviseSinks();
    if (FAILED(hr)) {
        UnadviseSinks();
        engine_.Stop();
        thread_mgr_->Release();
        thread_mgr_ = nullptr;
        client_id_ = TF_CLIENTID_NULL;
        return hr;
    }
    // Đổi input method có thể không phát OnSetFocus; lấy context hiện tại ngay.
    EnsureContext();
    return S_OK;
}

STDMETHODIMP TextService::Deactivate() {
    FlushCurrentContext();
    UnadviseSinks();
    ClearRouting();
    ReleaseContext();
    engine_.Stop();
    if (thread_mgr_ != nullptr) {
        thread_mgr_->Release();
        thread_mgr_ = nullptr;
    }
    client_id_ = TF_CLIENTID_NULL;
    return S_OK;
}

HRESULT TextService::AdviseSinks() {
    ITfSource* source = nullptr;
    HRESULT hr = thread_mgr_->QueryInterface(IID_ITfSource, reinterpret_cast<void**>(&source));
    if (FAILED(hr)) {
        return hr;
    }
    hr = source->AdviseSink(IID_ITfThreadMgrEventSink,
                            static_cast<ITfThreadMgrEventSink*>(this), &thread_sink_cookie_);
    source->Release();
    if (FAILED(hr)) {
        return hr;
    }

    ITfKeystrokeMgr* keystrokes = nullptr;
    hr = thread_mgr_->QueryInterface(IID_ITfKeystrokeMgr, reinterpret_cast<void**>(&keystrokes));
    if (FAILED(hr)) {
        return hr;
    }
    hr = keystrokes->AdviseKeyEventSink(client_id_, static_cast<ITfKeyEventSink*>(this), TRUE);
    keystrokes->Release();
    if (FAILED(hr)) {
        return hr;
    }
    key_sink_cookie_ = 1;
    return S_OK;
}

void TextService::UnadviseSinks() {
    if (thread_mgr_ == nullptr) {
        return;
    }
    if (thread_sink_cookie_ != 0) {
        ITfSource* source = nullptr;
        if (SUCCEEDED(thread_mgr_->QueryInterface(IID_ITfSource,
                                                  reinterpret_cast<void**>(&source)))) {
            source->UnadviseSink(thread_sink_cookie_);
            source->Release();
        }
        thread_sink_cookie_ = 0;
    }
    if (key_sink_cookie_ != 0) {
        ITfKeystrokeMgr* keystrokes = nullptr;
        if (SUCCEEDED(thread_mgr_->QueryInterface(IID_ITfKeystrokeMgr,
                                                  reinterpret_cast<void**>(&keystrokes)))) {
            keystrokes->UnadviseKeyEventSink(client_id_);
            keystrokes->Release();
        }
        key_sink_cookie_ = 0;
    }
}

STDMETHODIMP TextService::OnInitDocumentMgr(ITfDocumentMgr*) {
    return S_OK;
}

STDMETHODIMP TextService::OnUninitDocumentMgr(ITfDocumentMgr* pdim) {
    if (doc_mgr_ == pdim) {
        FlushCurrentContext();
        ReleaseContext();
        ClearRouting();
    }
    return S_OK;
}

STDMETHODIMP TextService::OnSetFocus(ITfDocumentMgr* pdimFocus, ITfDocumentMgr*) {
    if (thread_mgr_ == nullptr) {
        return S_OK;
    }
    FlushCurrentContext();
    ReleaseContext();

    if (pdimFocus == nullptr) {
        ClearRouting();
        engine_.FocusChanged("");
        return S_OK;
    }

    AttachContext(pdimFocus);
    return S_OK;
}

STDMETHODIMP TextService::OnPushContext(ITfContext*) {
    return S_OK;
}

STDMETHODIMP TextService::OnPopContext(ITfContext* pic) {
    if (context_ == pic) {
        FlushCurrentContext();
        ReleaseContext();
        ClearRouting();
    }
    return S_OK;
}

STDMETHODIMP TextService::OnSetFocus(BOOL) {
    return S_OK;
}

STDMETHODIMP TextService::OnTestKeyDown(ITfContext*, WPARAM wParam, LPARAM, BOOL* pfEaten) {
    if (pfEaten == nullptr) {
        return E_POINTER;
    }
    *pfEaten = FALSE;
    if (!BackendAllowsTsf() || !EnsureContext()) {
        return S_OK;
    }

    const bool ctrl = IsControlDown();
    const bool shift = IsShiftDown();
    if (IsToggleKey(wParam, ctrl, shift)) {
        *pfEaten = TRUE;
        return S_OK;
    }
    if (ctrl || IsAltDown() || IsMetaDown()) {
        return S_OK;
    }
    if (IsEngineKey(wParam)) {
        *pfEaten = TRUE;
    }
    return S_OK;
}

STDMETHODIMP TextService::OnKeyDown(ITfContext* pic, WPARAM wParam, LPARAM lParam,
                                    BOOL* pfEaten) {
    if (pfEaten == nullptr) {
        return E_POINTER;
    }
    *pfEaten = FALSE;
    if (pic == nullptr || !BackendAllowsTsf() || !EnsureContext()) {
        return S_OK;
    }

    const bool ctrl = IsControlDown();
    const bool shift = IsShiftDown();
    const bool alt = IsAltDown();
    const bool meta = IsMetaDown();
    if (IsToggleKey(wParam, ctrl, shift)) {
        if (SUCCEEDED(ExecuteToggle())) {
            *pfEaten = TRUE;
        }
        return S_OK;
    }
    if (ctrl || alt || meta) {
        return S_OK;
    }

    std::string key = KeyFromVk(wParam, lParam);
    if (key.empty()) {
        return S_OK;
    }

    auto* session = new (std::nothrow) KeyEditSession(this, pic, std::move(key), ctrl, alt, meta);
    if (session == nullptr) {
        return E_OUTOFMEMORY;
    }
    HRESULT session_result = S_OK;
    HRESULT hr = pic->RequestEditSession(client_id_, session, TF_ES_SYNC | TF_ES_READWRITE,
                                         &session_result);
    if (SUCCEEDED(hr)) {
        hr = session_result;
    }
    if (SUCCEEDED(hr) && session->eaten()) {
        *pfEaten = TRUE;
    }
    session->Release();
    return S_OK;
}

STDMETHODIMP TextService::OnTestKeyUp(ITfContext*, WPARAM, LPARAM, BOOL* pfEaten) {
    if (pfEaten == nullptr) {
        return E_POINTER;
    }
    *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP TextService::OnKeyUp(ITfContext*, WPARAM, LPARAM, BOOL* pfEaten) {
    if (pfEaten == nullptr) {
        return E_POINTER;
    }
    *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP TextService::OnPreservedKey(ITfContext*, REFGUID, BOOL* pfEaten) {
    if (pfEaten == nullptr) {
        return E_POINTER;
    }
    *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP TextService::OnCompositionTerminated(TfEditCookie, ITfComposition* pComposition) {
    if (ending_composition_) {
        return S_OK;
    }
    if (composition_ == pComposition) {
        composition_->Release();
        composition_ = nullptr;
        engine_.Reset();
    }
    return S_OK;
}

STDMETHODIMP TextService::EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo** ppEnum) {
    if (ppEnum == nullptr) {
        return E_POINTER;
    }
    *ppEnum = new (std::nothrow) ::EnumDisplayAttributeInfo();
    return *ppEnum == nullptr ? E_OUTOFMEMORY : S_OK;
}

STDMETHODIMP TextService::GetDisplayAttributeInfo(REFGUID guid,
                                                  ITfDisplayAttributeInfo** ppInfo) {
    if (ppInfo == nullptr) {
        return E_POINTER;
    }
    *ppInfo = nullptr;
    if (!IsEqualGUID(guid, GUID_ViFastKeyDisplayAttribute)) {
        return E_INVALIDARG;
    }
    *ppInfo = new (std::nothrow) DisplayAttributeInfo();
    return *ppInfo == nullptr ? E_OUTOFMEMORY : S_OK;
}

void TextService::ReleaseContext() {
    if (composition_ != nullptr) {
        composition_->Release();
        composition_ = nullptr;
    }
    if (context_ != nullptr) {
        context_->Release();
        context_ = nullptr;
    }
    if (doc_mgr_ != nullptr) {
        doc_mgr_->Release();
        doc_mgr_ = nullptr;
    }
}

bool TextService::AttachContext(ITfDocumentMgr* pdimFocus) {
    if (thread_mgr_ == nullptr || pdimFocus == nullptr) {
        return false;
    }
    ITfContext* context = nullptr;
    if (FAILED(pdimFocus->GetTop(&context)) || context == nullptr) {
        ClearRouting();
        return false;
    }

    context_ = context;  // GetTop đã AddRef
    doc_mgr_ = pdimFocus;
    doc_mgr_->AddRef();

    ReloadConfigIfNeeded();
    UpdateAppAndField();

    if (BackendAllowsTsf()) {
        PublishRouting();
    } else {
        ClearRouting();
    }
    return true;
}

bool TextService::EnsureContext() {
    if (context_ != nullptr) {
        return true;
    }
    if (thread_mgr_ == nullptr) {
        return false;
    }
    ITfDocumentMgr* doc = nullptr;
    if (FAILED(thread_mgr_->GetFocus(&doc)) || doc == nullptr) {
        return false;
    }
    const bool ok = AttachContext(doc);
    doc->Release();
    return ok;
}

void TextService::PublishRouting() {
    PublishTsfActive();
}

void TextService::ClearRouting() {
    ClearTsfActive();
}

bool TextService::BackendAllowsTsf() const {
    return backend_ != "hook";
}

void TextService::ReloadConfigIfNeeded() {
    AppConfig config = LoadAppConfig();
    backend_ = config.backend;
    if (config.engine_json.empty() || config.engine_json == loaded_config_) {
        return;
    }
    ActionData action;
    if (engine_.Reload(config.engine_json, &action)) {
        loaded_config_ = config.engine_json;
        ApplyActionOn(context_, action);
    }
}

HRESULT TextService::ApplyActionOn(ITfContext* context, const ActionData& action) {
    if (context == nullptr || action.kind == VFK_PASS) {
        return S_OK;
    }
    if (client_id_ == TF_CLIENTID_NULL) {
        return E_UNEXPECTED;
    }
    auto* session = new (std::nothrow) ActionEditSession(this, context, action);
    if (session == nullptr) {
        return E_OUTOFMEMORY;
    }
    HRESULT session_result = S_OK;
    HRESULT hr = context->RequestEditSession(client_id_, session, TF_ES_SYNC | TF_ES_READWRITE,
                                             &session_result);
    if (SUCCEEDED(hr)) {
        hr = session_result;
    }
    session->Release();
    return hr;
}

HRESULT TextService::FlushCurrentContext() {
    if (context_ == nullptr) {
        return S_OK;
    }
    ActionData action = engine_.Flush();
    if (action.kind == VFK_PASS) {
        return S_OK;
    }
    return ApplyActionOn(context_, action);
}

HRESULT TextService::ExecuteToggle() {
    ActionData action = engine_.Toggle();
    return ApplyActionOn(context_, action);
}

HRESULT TextService::UpdateAppAndField() {
    if (context_ == nullptr) {
        return S_OK;
    }
    ActionData focus_action = engine_.FocusChanged(AppIdFromForeground());
    ApplyActionOn(context_, focus_action);

    uint32_t field = VFK_FIELD_TEXT;
    auto* session = new (std::nothrow) FieldEditSession(this, context_);
    if (session != nullptr) {
        HRESULT session_result = S_OK;
        HRESULT hr = context_->RequestEditSession(client_id_, session, TF_ES_SYNC | TF_ES_READ,
                                                  &session_result);
        if (SUCCEEDED(hr) && SUCCEEDED(session_result)) {
            field = session->field();
        }
        session->Release();
    }

    ActionData field_action = engine_.SetField(field);
    return ApplyActionOn(context_, field_action);
}

HRESULT TextService::HandleAction(ITfContext* context, TfEditCookie ec,
                                   const ActionData& action, bool* eaten) {
    if (eaten != nullptr) {
        *eaten = false;
    }
    SurroundingText surrounding = GetSurrounding(context, ec);
    engine_.SetSurrounding(surrounding.utf8);
    PlanData plan = engine_.Plan(action, surrounding.utf8, surrounding.utf16_len,
                                 surrounding.utf16_len, false);
    return ApplyPlan(context, ec, plan, eaten);
}

HRESULT TextService::HandleKey(ITfContext* context, TfEditCookie ec, const std::string& key,
                               bool ctrl, bool alt, bool meta, bool* eaten) {
    if (eaten != nullptr) {
        *eaten = false;
    }
    SurroundingText surrounding = GetSurrounding(context, ec);
    const uint32_t field = ReadFieldKind(context, ec);
    if (field != engine_.field_kind()) {
        ActionData change = engine_.SetField(field);
        if (change.kind != VFK_PASS) {
            PlanData change_plan = engine_.Plan(change, surrounding.utf8, surrounding.utf16_len,
                                                surrounding.utf16_len, false);
            bool ignored = false;
            ApplyPlan(context, ec, change_plan, &ignored);
            surrounding = GetSurrounding(context, ec);
        }
    }
    engine_.SetSurrounding(surrounding.utf8);

    ActionData action = engine_.ProcessKey(key, ctrl, alt, meta);
    if (action.kind == VFK_PASS) {
        return S_OK;
    }
    PlanData plan = engine_.Plan(action, surrounding.utf8, surrounding.utf16_len,
                                 surrounding.utf16_len, false);
    return ApplyPlan(context, ec, plan, eaten);
}

HRESULT TextService::ApplyPlan(ITfContext* context, TfEditCookie ec, const PlanData& plan,
                               bool* eaten) {
    if (eaten != nullptr) {
        *eaten = false;
    }
    switch (plan.kind) {
        case VFK_PLAN_PASS:
            return S_OK;
        case VFK_PLAN_PREEDIT: {
            HRESULT hr = SetPreedit(context, ec, Utf8ToWide(plan.text), plan.cursor_utf16);
            if (FAILED(hr)) {
                EndComposition(ec);
                engine_.Reset();
                return hr;
            }
            if (eaten != nullptr) {
                *eaten = true;
            }
            return S_OK;
        }
        case VFK_PLAN_COMMIT: {
            HRESULT hr = CommitText(context, ec, Utf8ToWide(plan.text));
            if (FAILED(hr)) {
                EndComposition(ec);
                engine_.Reset();
                return hr;
            }
            if (eaten != nullptr) {
                *eaten = !plan.pass_key;
            }
            return S_OK;
        }
        case VFK_PLAN_REPLACE: {
            HRESULT hr =
                ReplaceBeforeCaret(context, ec, plan.delete_utf16, Utf8ToWide(plan.text));
            if (hr != S_OK) {
                EndComposition(ec);
                engine_.Reset();
                return S_FALSE;
            }
            if (eaten != nullptr) {
                *eaten = !plan.pass_key;
            }
            return S_OK;
        }
        case VFK_PLAN_FAIL_SAFE:
        default: {
            EndComposition(ec);
            engine_.Reset();
            return S_OK;
        }
    }
}

HRESULT TextService::SetPreedit(ITfContext* context, TfEditCookie ec,
                                const std::wstring& text, uint32_t cursor_utf16) {
    if (composition_ == nullptr) {
        if (text.empty()) {
            return S_OK;
        }
        TF_SELECTION selection = {};
        ULONG count = 0;
        HRESULT hr =
            context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &count);
        if (FAILED(hr) || count == 0) {
            return FAILED(hr) ? hr : E_FAIL;
        }
        BOOL empty = FALSE;
        if (SUCCEEDED(selection.range->IsEmpty(ec, &empty)) && !empty) {
            // Thay vùng chọn bằng composition mới, giống hành vi gõ đè.
            selection.range->SetText(ec, 0, L"", 0);
            selection.range->Collapse(ec, TF_ANCHOR_START);
        }
        ITfContextComposition* composition_context = nullptr;
        hr = context->QueryInterface(IID_ITfContextComposition,
                                     reinterpret_cast<void**>(&composition_context));
        if (FAILED(hr) || composition_context == nullptr) {
            selection.range->Release();
            return FAILED(hr) ? hr : E_FAIL;
        }
        hr = composition_context->StartComposition(
            ec, selection.range, static_cast<ITfCompositionSink*>(this), &composition_);
        composition_context->Release();
        selection.range->Release();
        if (FAILED(hr) || composition_ == nullptr) {
            composition_ = nullptr;
            return FAILED(hr) ? hr : E_FAIL;
        }
    }

    ITfRange* range = nullptr;
    HRESULT hr = composition_->GetRange(&range);
    if (FAILED(hr) || range == nullptr) {
        return FAILED(hr) ? hr : E_FAIL;
    }

    if (text.empty()) {
        hr = range->SetText(ec, 0, L"", 0);
        range->Release();
        if (SUCCEEDED(hr)) {
            EndComposition(ec);
        }
        return hr;
    }

    hr = range->SetText(ec, 0, text.c_str(), static_cast<LONG>(text.size()));
    if (SUCCEEDED(hr)) {
        ApplyDisplayAttribute(context, ec, range);
        SetCaret(context, ec, range, cursor_utf16);
    }
    range->Release();
    return hr;
}

HRESULT TextService::CommitText(ITfContext* context, TfEditCookie ec,
                                const std::wstring& text) {
    if (composition_ == nullptr) {
        return InsertAtCaret(context, ec, text);
    }

    ITfRange* range = nullptr;
    HRESULT hr = composition_->GetRange(&range);
    if (FAILED(hr) || range == nullptr) {
        return FAILED(hr) ? hr : E_FAIL;
    }
    const wchar_t* data = text.empty() ? L"" : text.c_str();
    hr = range->SetText(ec, 0, data, static_cast<LONG>(text.size()));
    if (FAILED(hr)) {
        range->Release();
        return hr;
    }

    // Notepad (RichEdit) đưa caret về đầu composition khi EndComposition. Giữ
    // một range độc lập bao text vừa commit, kết thúc composition rồi đặt caret
    // về cuối range đó. Edge/VSCode không cần nhưng thao tác này vô hại.
    ITfRange* after = nullptr;
    hr = range->Clone(&after);
    range->Release();
    if (FAILED(hr) || after == nullptr) {
        return EndComposition(ec);
    }

    after->Collapse(ec, TF_ANCHOR_END);
    TF_SELECTION before_end = {after, TF_AE_END};
    context->SetSelection(ec, 1, &before_end);

    const HRESULT end_hr = EndComposition(ec);
    if (SUCCEEDED(end_hr)) {
        after->Collapse(ec, TF_ANCHOR_END);
        TF_SELECTION out = {after, TF_AE_END};
        context->SetSelection(ec, 1, &out);
    }
    after->Release();
    return end_hr;
}

HRESULT TextService::EndComposition(TfEditCookie ec) {
    if (composition_ == nullptr) {
        return S_OK;
    }
    ending_composition_ = true;
    HRESULT hr = composition_->EndComposition(ec);
    ending_composition_ = false;
    composition_->Release();
    composition_ = nullptr;
    return hr;
}

HRESULT TextService::ReplaceBeforeCaret(ITfContext* context, TfEditCookie ec,
                                        uint32_t delete_utf16, const std::wstring& text) {
    if (delete_utf16 > static_cast<uint32_t>(LONG_MAX)) {
        return E_INVALIDARG;
    }
    TF_SELECTION selection = {};
    ULONG count = 0;
    HRESULT hr = context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &count);
    if (FAILED(hr) || count == 0) {
        return FAILED(hr) ? hr : E_FAIL;
    }

    BOOL empty = FALSE;
    hr = selection.range->IsEmpty(ec, &empty);
    if (FAILED(hr) || !empty) {
        selection.range->Release();
        return S_FALSE;
    }

    if (delete_utf16 > 0) {
        LONG shifted = 0;
        const LONG request = -static_cast<LONG>(delete_utf16);
        hr = selection.range->ShiftStart(ec, request, &shifted, nullptr);
        if (FAILED(hr) || shifted != request) {
            selection.range->Release();
            return S_FALSE;
        }
    }

    hr = selection.range->SetText(ec, 0, text.c_str(), static_cast<LONG>(text.size()));
    if (SUCCEEDED(hr)) {
        selection.range->Collapse(ec, TF_ANCHOR_END);
        TF_SELECTION out = {selection.range, TF_AE_END};
        context->SetSelection(ec, 1, &out);
    }
    selection.range->Release();
    return hr;
}

HRESULT TextService::InsertAtCaret(ITfContext* context, TfEditCookie ec,
                                   const std::wstring& text) {
    TF_SELECTION selection = {};
    ULONG count = 0;
    HRESULT hr = context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &count);
    if (FAILED(hr) || count == 0) {
        return FAILED(hr) ? hr : E_FAIL;
    }
    const wchar_t* data = text.empty() ? L"" : text.c_str();
    hr = selection.range->SetText(ec, 0, data, static_cast<LONG>(text.size()));
    if (SUCCEEDED(hr)) {
        selection.range->Collapse(ec, TF_ANCHOR_END);
        TF_SELECTION out = {selection.range, TF_AE_END};
        context->SetSelection(ec, 1, &out);
    }
    selection.range->Release();
    return hr;
}

void TextService::SetCaret(ITfContext* context, TfEditCookie ec, ITfRange* range,
                           uint32_t cursor_utf16) {
    ITfRange* caret = nullptr;
    if (FAILED(range->Clone(&caret)) || caret == nullptr) {
        return;
    }
    caret->Collapse(ec, TF_ANCHOR_START);
    if (cursor_utf16 > 0) {
        LONG shifted = 0;
        caret->ShiftEnd(ec, static_cast<LONG>(cursor_utf16), &shifted, nullptr);
    }
    caret->Collapse(ec, TF_ANCHOR_END);
    TF_SELECTION selection = {caret, TF_AE_END};
    context->SetSelection(ec, 1, &selection);
    caret->Release();
}

void TextService::ApplyDisplayAttribute(ITfContext* context, TfEditCookie ec, ITfRange* range) {
    static TfGuidAtom atom = 0;
    if (atom == 0) {
        ITfCategoryMgr* categories = nullptr;
        if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                                       IID_ITfCategoryMgr,
                                       reinterpret_cast<void**>(&categories)))) {
            categories->RegisterGUID(GUID_ViFastKeyDisplayAttribute, &atom);
            categories->Release();
        }
    }
    if (atom == 0) {
        return;
    }
    ITfProperty* property = nullptr;
    if (FAILED(context->GetProperty(GUID_PROP_ATTRIBUTE, &property)) || property == nullptr) {
        return;
    }
    VARIANT value;
    VariantInit(&value);
    value.vt = VT_I4;
    value.lVal = atom;
    property->SetValue(ec, range, &value);
    VariantClear(&value);
    property->Release();
}

SurroundingText TextService::GetSurrounding(ITfContext* context, TfEditCookie ec) const {
    SurroundingText result;
    if (context == nullptr) {
        return result;
    }
    TF_SELECTION selection = {};
    ULONG count = 0;
    HRESULT hr = context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &count);
    if (FAILED(hr) || count == 0) {
        return result;
    }

    BOOL empty = FALSE;
    hr = selection.range->IsEmpty(ec, &empty);
    if (FAILED(hr) || !empty) {
        selection.range->Release();
        return result;
    }

    ITfRange* before = nullptr;
    hr = selection.range->Clone(&before);
    selection.range->Release();
    if (FAILED(hr) || before == nullptr) {
        return result;
    }

    before->Collapse(ec, TF_ANCHOR_START);
    LONG shifted = 0;
    before->ShiftStart(ec, -kMaxSurrounding, &shifted, nullptr);

    wchar_t buffer[static_cast<size_t>(kMaxSurrounding) + 1] = {};
    ULONG fetched = 0;
    hr = before->GetText(ec, 0, buffer, static_cast<ULONG>(kMaxSurrounding), &fetched);
    before->Release();
    if (FAILED(hr)) {
        return result;
    }

    std::wstring text(buffer, fetched);
    result.utf8 = WideToUtf8(text);
    result.utf16_len = Utf16Len(text);
    return result;
}

uint32_t TextService::ReadFieldKind(ITfContext* context, TfEditCookie ec) const {
    uint32_t result = VFK_FIELD_TEXT;
    if (context == nullptr) {
        return result;
    }

    ITfProperty* property = nullptr;
    if (FAILED(context->GetProperty(kGuidPropInputScope, &property)) || property == nullptr) {
        if (IsTerminalApp(AppIdFromForeground())) {
            result = VFK_FIELD_TERMINAL;
        }
        return result;
    }

    TF_SELECTION selection = {};
    ULONG count = 0;
    HRESULT hr = context->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &selection, &count);
    if (FAILED(hr) || count == 0) {
        property->Release();
        if (IsTerminalApp(AppIdFromForeground())) {
            result = VFK_FIELD_TERMINAL;
        }
        return result;
    }

    VARIANT var;
    VariantInit(&var);
    hr = property->GetValue(ec, selection.range, &var);
    if (SUCCEEDED(hr) && var.vt == VT_UNKNOWN && var.punkVal != nullptr) {
        ITfInputScope* scope = nullptr;
        if (SUCCEEDED(var.punkVal->QueryInterface(IID_ITfInputScope,
                                                  reinterpret_cast<void**>(&scope))) &&
            scope != nullptr) {
            InputScope* scopes = nullptr;
            UINT scope_count = 0;
            if (SUCCEEDED(scope->GetInputScopes(&scopes, &scope_count)) && scopes != nullptr) {
                for (UINT i = 0; i < scope_count; ++i) {
                    switch (scopes[i]) {
                        case IS_PASSWORD:
                        case IS_NUMERIC_PASSWORD:
                        case IS_NUMERIC_PIN:
                        case IS_ALPHANUMERIC_PIN:
                        case IS_ALPHANUMERIC_PIN_SET:
                            result = VFK_FIELD_PASSWORD;
                            break;
                        case IS_URL:
                            if (result != VFK_FIELD_PASSWORD) {
                                result = VFK_FIELD_URL;
                            }
                            break;
                        case IS_EMAIL_SMTPEMAILADDRESS:
                        case IS_EMAILNAME_OR_ADDRESS:
                            if (result == VFK_FIELD_TEXT || result == VFK_FIELD_URL) {
                                result = VFK_FIELD_EMAIL;
                            }
                            break;
                        case IS_NUMBER:
                        case IS_DIGITS:
                        case IS_TELEPHONE_FULLTELEPHONENUMBER:
                        case IS_TELEPHONE_LOCALNUMBER:
                        case IS_TELEPHONE_AREACODE:
                        case IS_TELEPHONE_COUNTRYCODE:
                            if (result == VFK_FIELD_TEXT) {
                                result = VFK_FIELD_NUMBER;
                            }
                            break;
                        default:
                            break;
                    }
                }
                CoTaskMemFree(scopes);
            }
            scope->Release();
        }
    }
    VariantClear(&var);
    selection.range->Release();
    property->Release();

    if (result == VFK_FIELD_TEXT && IsTerminalApp(AppIdFromForeground())) {
        result = VFK_FIELD_TERMINAL;
    }
    return result;
}

std::string TextService::AppIdFromForeground() const {
    HWND window = GetForegroundWindow();
    if (window == nullptr) {
        return {};
    }
    DWORD pid = 0;
    GetWindowThreadProcessId(window, &pid);
    if (pid == 0) {
        return {};
    }
    HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (process == nullptr) {
        return {};
    }
    wchar_t path[1024] = {};
    DWORD size = static_cast<DWORD>(std::size(path));
    const BOOL ok = QueryFullProcessImageNameW(process, 0, path, &size);
    CloseHandle(process);
    if (ok == FALSE) {
        return {};
    }

    std::wstring name(path, size);
    const size_t slash = name.find_last_of(L"\\/");
    if (slash != std::wstring::npos) {
        name = name.substr(slash + 1);
    }
    if (name.size() > 4) {
        std::wstring extension = name.substr(name.size() - 4);
        for (wchar_t& ch : extension) {
            ch = static_cast<wchar_t>(std::towlower(ch));
        }
        if (extension == L".exe") {
            name.resize(name.size() - 4);
        }
    }
    for (wchar_t& ch : name) {
        ch = static_cast<wchar_t>(std::towlower(ch));
    }
    return WideToUtf8(name);
}

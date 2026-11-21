#include "text_service.h"

#include <oleauto.h>

#include <new>
#include <vector>

#include "display_attribute.h"
#include "rustmagic_ids.h"

namespace rustmagic {
namespace {

/* Drops the last `count` characters from `text`, surrogate pairs counted once. */
void drop_characters(std::wstring &text, size_t count) {
    for (size_t i = 0; i < count && !text.empty(); ++i) {
        text.pop_back();
        if (!text.empty() && text.back() >= 0xDC00 && text.back() <= 0xDFFF && text.size() >= 2) {
            const wchar_t high = text[text.size() - 2];
            if (high >= 0xD800 && high <= 0xDBFF) {
                text.pop_back();
            }
        }
    }
}

/* The TfGuidAtom that stands for our display attribute inside the document.
 * The category manager mints it once per process and every context uses the
 * same number, which is how a text service marks text it has not committed. */
TfGuidAtom display_attribute_atom() {
    static TfGuidAtom atom = 0;
    if (atom == 0) {
        ITfCategoryMgr *categories = nullptr;
        if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                                       IID_ITfCategoryMgr,
                                       reinterpret_cast<void **>(&categories))) &&
            categories != nullptr) {
            categories->RegisterGUID(GUID_RustMagicDisplayAttribute, &atom);
            categories->Release();
        }
    }
    return atom;
}

/* The edit session that writes the composition. TSF gives text services a
 * cookie only inside DoEditSession, and only through the application's thread,
 * so every document change goes through here. */
class CompositionEditSession : public ITfEditSession {
public:
    CompositionEditSession(TextService *owner, ITfContext *context, std::wstring text,
                           bool finish)
        : owner_(owner), context_(context), text_(std::move(text)), finish_(finish) {
        owner_->AddRef();
        context_->AddRef();
    }

    /* IUnknown */
    STDMETHODIMP QueryInterface(REFIID riid, void **ppv) override {
        if (ppv == nullptr) {
            return E_INVALIDARG;
        }
        *ppv = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfEditSession)) {
            *ppv = static_cast<ITfEditSession *>(this);
        } else {
            return E_NOINTERFACE;
        }
        AddRef();
        return S_OK;
    }
    STDMETHODIMP_(ULONG) AddRef() override { return InterlockedIncrement(&refs_); }
    STDMETHODIMP_(ULONG) Release() override {
        const LONG refs = InterlockedDecrement(&refs_);
        if (refs == 0) {
            delete this;
        }
        return refs;
    }

    STDMETHODIMP DoEditSession(TfEditCookie ec) override {
        return owner_->write_composition(context_, ec, text_, finish_);
    }

private:
    ~CompositionEditSession() {
        context_->Release();
        owner_->Release();
    }

    LONG refs_ = 1;
    TextService *owner_;
    ITfContext *context_;
    std::wstring text_;
    bool finish_;
};

} // namespace

TextService::TextService() = default;

STDMETHODIMP TextService::QueryInterface(REFIID riid, void **ppv) {
    if (ppv == nullptr) {
        return E_INVALIDARG;
    }
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfTextInputProcessor) ||
        IsEqualIID(riid, IID_ITfTextInputProcessorEx)) {
        *ppv = static_cast<ITfTextInputProcessorEx *>(this);
    } else if (IsEqualIID(riid, IID_ITfThreadMgrEventSink)) {
        *ppv = static_cast<ITfThreadMgrEventSink *>(this);
    } else if (IsEqualIID(riid, IID_ITfKeyEventSink)) {
        *ppv = static_cast<ITfKeyEventSink *>(this);
    } else if (IsEqualIID(riid, IID_ITfCompositionSink)) {
        *ppv = static_cast<ITfCompositionSink *>(this);
    } else if (IsEqualIID(riid, IID_ITfDisplayAttributeProvider)) {
        *ppv = static_cast<ITfDisplayAttributeProvider *>(this);
    } else {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) TextService::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) TextService::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (refs == 0) {
        delete this;
    }
    return refs;
}

/* ---------------------------------------------------------------- activation */

STDMETHODIMP TextService::Activate(ITfThreadMgr *thread_mgr, TfClientId client_id) {
    return ActivateEx(thread_mgr, client_id, 0);
}

STDMETHODIMP TextService::ActivateEx(ITfThreadMgr *thread_mgr, TfClientId client_id, DWORD) {
    if (thread_mgr == nullptr) {
        return E_INVALIDARG;
    }
    thread_mgr_ = thread_mgr;
    thread_mgr_->AddRef();
    client_id_ = client_id;

    /* The keys come through the keystroke manager; the focus changes come
     * through the thread manager itself. Both are advised here and released in
     * Deactivate, in this order, or TSF keeps calling into a dead object. */
    ITfKeystrokeMgr *keystrokes = nullptr;
    if (SUCCEEDED(thread_mgr_->QueryInterface(IID_ITfKeystrokeMgr,
                                              reinterpret_cast<void **>(&keystrokes))) &&
        keystrokes != nullptr) {
        keystrokes->AdviseKeyEventSink(client_id_, static_cast<ITfKeyEventSink *>(this), TRUE);
        keystrokes->Release();
    }

    ITfSource *source = nullptr;
    if (SUCCEEDED(thread_mgr_->QueryInterface(IID_ITfSource, reinterpret_cast<void **>(&source))) &&
        source != nullptr) {
        source->AdviseSink(IID_ITfThreadMgrEventSink,
                           static_cast<ITfThreadMgrEventSink *>(this), &thread_mgr_cookie_);
        source->Release();
    }

    engine_.reload();
    return S_OK;
}

STDMETHODIMP TextService::Deactivate() {
    /* A composition left open would underline text in an application that no
     * longer has our input method; commit what the user typed before leaving. */
    commit_and_reset(composition_context_, true);

    if (thread_mgr_ != nullptr) {
        ITfSource *source = nullptr;
        if (SUCCEEDED(thread_mgr_->QueryInterface(IID_ITfSource,
                                                  reinterpret_cast<void **>(&source))) &&
            source != nullptr) {
            if (thread_mgr_cookie_ != TF_INVALID_COOKIE) {
                source->UnadviseSink(thread_mgr_cookie_);
                thread_mgr_cookie_ = TF_INVALID_COOKIE;
            }
            source->Release();
        }
        ITfKeystrokeMgr *keystrokes = nullptr;
        if (SUCCEEDED(thread_mgr_->QueryInterface(IID_ITfKeystrokeMgr,
                                                  reinterpret_cast<void **>(&keystrokes))) &&
            keystrokes != nullptr) {
            keystrokes->UnadviseKeyEventSink(client_id_);
            keystrokes->Release();
        }
        thread_mgr_->Release();
        thread_mgr_ = nullptr;
    }
    client_id_ = TF_CLIENTID_NULL;
    return S_OK;
}

/* ------------------------------------------------------------ focus changes */

STDMETHODIMP TextService::OnInitDocumentMgr(ITfDocumentMgr *) { return S_OK; }

STDMETHODIMP TextService::OnUninitDocumentMgr(ITfDocumentMgr *) { return S_OK; }

STDMETHODIMP TextService::OnSetFocus(ITfDocumentMgr *focused, ITfDocumentMgr *previous) {
    if (previous != nullptr && composition_context_ != nullptr) {
        /* Commit into the window we are leaving, not the one we arrive in. */
        commit_and_reset(composition_context_, true);
    }
    if (focused != nullptr) {
        engine_.reload();
    }
    return S_OK;
}

STDMETHODIMP TextService::OnPushContext(ITfContext *) { return S_OK; }

STDMETHODIMP TextService::OnPopContext(ITfContext *) { return S_OK; }

/* --------------------------------------------------------------- the keys */

STDMETHODIMP TextService::OnSetFocus(BOOL) { return S_OK; }

/* TSF asks this before it processes a key: if the answer is "not mine" the
 * application gets the key and OnKeyDown is never called. So the answer has to
 * be the engine's own, worked out on a copy of its state: a service that claims
 * every key swallows the application's typing, and one that claims none never
 * types at all. */
STDMETHODIMP TextService::OnTestKeyDown(ITfContext *context, WPARAM wparam, LPARAM lparam,
                                       BOOL *eaten) {
    if (eaten == nullptr) {
        return E_INVALIDARG;
    }
    *eaten = FALSE;
    if (context == nullptr) {
        return S_OK;
    }
    if (!engine_.ready()) {
        engine_.reload();
    }

    const UINT vk = static_cast<UINT>(wparam);
    if (vk == VK_SHIFT || vk == VK_CONTROL || vk == VK_MENU || vk == VK_CAPITAL ||
        vk == VK_LWIN || vk == VK_RWIN) {
        return S_OK;
    }

    const bool shift = (GetKeyState(VK_SHIFT) & 0x8000) != 0;
    const bool control = (GetKeyState(VK_CONTROL) & 0x8000) != 0;
    const bool right_alt = (GetKeyState(VK_RMENU) & 0x8000) != 0;
    const bool left_alt = (GetKeyState(VK_LMENU) & 0x8000) != 0;
    const bool alt_gr = right_alt && control;
    const bool alt = left_alt && !alt_gr;
    const bool caps = (GetKeyState(VK_CAPITAL) & 1) != 0;

    /* The keys the service handles itself, whatever the engine would say. */
    if (vk == VK_ESCAPE) {
        *eaten = composing_.empty() ? FALSE : TRUE;
        return S_OK;
    }
    if (vk == VK_RETURN || vk == VK_TAB || vk == VK_SPACE || control || (left_alt && !alt_gr) ||
        vk == VK_LEFT || vk == VK_RIGHT || vk == VK_UP || vk == VK_DOWN || vk == VK_HOME ||
        vk == VK_END || vk == VK_PRIOR || vk == VK_NEXT || vk == VK_DELETE || vk == VK_INSERT) {
        return S_OK;
    }
    if (vk == VK_BACK) {
        /* Inside a syllable Backspace is the engine's; outside it, the
         * application's. */
        *eaten = composing_.empty() ? FALSE : TRUE;
        return S_OK;
    }

    wchar_t typed = 0;
    BYTE state[256] = {};
    GetKeyboardState(state);
    WCHAR buffer[8] = {};
    const UINT scan = static_cast<UINT>((lparam >> 16) & 0xFF);
    if (ToUnicode(vk, scan, state, buffer, ARRAYSIZE(buffer), 0) > 0) {
        typed = buffer[0];
    }
    *eaten = engine_.would_consume(vk, typed, shift, control, alt, alt_gr, caps) ? TRUE : FALSE;
    return S_OK;
}

STDMETHODIMP TextService::OnTestKeyUp(ITfContext *, WPARAM, LPARAM, BOOL *eaten) {
    if (eaten != nullptr) {
        *eaten = FALSE;
    }
    return S_OK;
}

STDMETHODIMP TextService::OnKeyDown(ITfContext *context, WPARAM wparam, LPARAM lparam,
                                   BOOL *eaten) {
    if (eaten == nullptr || context == nullptr) {
        return E_INVALIDARG;
    }
    *eaten = FALSE;

    /* The window may have switched keyboards since activation; a key press is a
     * cheap moment to notice. */
    if (!engine_.ready()) {
        engine_.reload();
    }
    if (!engine_.ready()) {
        return S_OK;
    }

    const UINT vk = static_cast<UINT>(wparam);
    if (vk == VK_SHIFT || vk == VK_CONTROL || vk == VK_MENU || vk == VK_CAPITAL ||
        vk == VK_LWIN || vk == VK_RWIN) {
        return S_OK;
    }

    const bool shift = (GetKeyState(VK_SHIFT) & 0x8000) != 0;
    const bool control = (GetKeyState(VK_CONTROL) & 0x8000) != 0;
    const bool right_alt = (GetKeyState(VK_RMENU) & 0x8000) != 0;
    const bool left_alt = (GetKeyState(VK_LMENU) & 0x8000) != 0;
    const bool alt_gr = right_alt && control;
    const bool alt = left_alt && !alt_gr;
    const bool caps = (GetKeyState(VK_CAPITAL) & 1) != 0;

    /* Escape throws away what is composing, the way every input method does. */
    if (vk == VK_ESCAPE) {
        if (!composing_.empty()) {
            composing_.clear();
            update_composition(composition_context_ != nullptr ? composition_context_ : context,
                               true);
            *eaten = TRUE;
        }
        return S_OK;
    }

    /* Enter, tab and space end the syllable and belong to the application. */
    if (vk == VK_RETURN || vk == VK_TAB || vk == VK_SPACE) {
        if (!composing_.empty()) {
            commit_and_reset(composition_context_ != nullptr ? composition_context_ : context,
                             true);
        }
        return S_OK;
    }

    /* Shortcuts are not text. Commit and let the application see the key. */
    if (control || (left_alt && !alt_gr) || vk == VK_LEFT || vk == VK_RIGHT || vk == VK_UP ||
        vk == VK_DOWN || vk == VK_HOME || vk == VK_END || vk == VK_PRIOR || vk == VK_NEXT ||
        vk == VK_DELETE || vk == VK_INSERT) {
        if (!composing_.empty()) {
            commit_and_reset(composition_context_ != nullptr ? composition_context_ : context,
                             true);
        }
        return S_OK;
    }

    /* Backspace: the engine may want it (a Myanmar syllable deletes by rule),
     * and when it does not, it deletes one character of the composition. With
     * nothing composing it belongs to the application. */
    if (vk == VK_BACK) {
        if (composing_.empty()) {
            return S_OK;
        }
        *eaten = TRUE;
        const Edit edit = engine_.process(vk, 0, shift, control, alt, alt_gr, caps);
        if (edit.consumed) {
            apply(context, edit, false);
        } else {
            drop_characters(composing_, 1);
            update_composition(context, composing_.empty());
        }
        return S_OK;
    }

    /* An ordinary key. The character comes from the keyboard layout the user has
     * selected in Windows, which is what the engine's `$latin` tables expect. */
    wchar_t typed = 0;
    {
        BYTE state[256] = {};
        GetKeyboardState(state);
        WCHAR buffer[8] = {};
        const UINT scan = static_cast<UINT>((lparam >> 16) & 0xFF);
        const int produced = ToUnicode(static_cast<UINT>(wparam), scan, state, buffer,
                                       ARRAYSIZE(buffer), 0);
        if (produced > 0) {
            typed = buffer[0];
        }
    }

    const Edit edit = engine_.process(vk, typed, shift, control, alt, alt_gr, caps);
    if (!edit.consumed) {
        /* The engine has no rule: the key is ordinary text, so the syllable is
         * over and the key passes through untouched. */
        if (!composing_.empty()) {
            commit_and_reset(composition_context_ != nullptr ? composition_context_ : context,
                             true);
        }
        return S_OK;
    }

    *eaten = TRUE;
    apply(context, edit, false);
    return S_OK;
}

STDMETHODIMP TextService::OnKeyUp(ITfContext *, WPARAM, LPARAM, BOOL *eaten) {
    if (eaten != nullptr) {
        *eaten = FALSE;
    }
    return S_OK;
}

STDMETHODIMP TextService::OnPreservedKey(ITfContext *, REFGUID, BOOL *eaten) {
    if (eaten != nullptr) {
        *eaten = FALSE;
    }
    return S_OK;
}

STDMETHODIMP TextService::OnCompositionTerminated(TfEditCookie, ITfComposition *composition) {
    /* The application took the composition away (a window destroying itself, a
     * document closing). Forget it; the text is already in the document. */
    if (composition_ != nullptr && composition != nullptr) {
        composition_->Release();
    }
    composition_ = nullptr;
    composing_.clear();
    if (composition_context_ != nullptr) {
        composition_context_->Release();
        composition_context_ = nullptr;
    }
    engine_.reset();
    return S_OK;
}

/* ------------------------------------------------------------- the document */

void TextService::apply(ITfContext *context, const Edit &edit, bool finish) {
    std::wstring next = composing_;
    drop_characters(next, edit.erase);
    next += edit.insert;
    composing_ = next;
    update_composition(context, finish);
}

void TextService::update_composition(ITfContext *context, bool finish) {
    if (context == nullptr) {
        return;
    }
    CompositionEditSession *session =
        new (std::nothrow) CompositionEditSession(this, context, composing_, finish);
    if (session == nullptr) {
        return;
    }
    HRESULT result = E_FAIL;
    /* Synchronous: the user is waiting for the character to appear. */
    context->RequestEditSession(client_id_, session, TF_ES_SYNC | TF_ES_READWRITE, &result);
    session->Release();
    if (FAILED(result)) {
        /* The document refused the write. Drop the composition state so the next
         * key starts from what is really on screen. */
        composing_.clear();
        finish_cleanup();
    }
}

void TextService::finish_cleanup() {
    if (composition_ != nullptr) {
        composition_->Release();
        composition_ = nullptr;
    }
    if (composition_context_ != nullptr) {
        composition_context_->Release();
        composition_context_ = nullptr;
    }
    engine_.reset();
}

void TextService::commit_and_reset(ITfContext *context, bool commit) {
    if (commit && context != nullptr && composition_ != nullptr && !composing_.empty()) {
        update_composition(context, true);
    }
    composing_.clear();
    finish_cleanup();
    engine_.reset();
}

STDMETHODIMP TextService::write_composition(ITfContext *context, TfEditCookie ec,
                                            const std::wstring &text, bool finish) {
    if (context == nullptr) {
        return E_INVALIDARG;
    }

    if (composition_ == nullptr) {
        if (text.empty()) {
            return S_OK; /* nothing to write and nothing to end */
        }
        ITfInsertAtSelection *insert = nullptr;
        if (FAILED(context->QueryInterface(IID_ITfInsertAtSelection,
                                           reinterpret_cast<void **>(&insert))) ||
            insert == nullptr) {
            return E_NOINTERFACE;
        }
        ITfRange *range = nullptr;
        /* QUERYONLY leaves the document alone and hands back the range at the
         * caret, which is where the composition is anchored. */
        const HRESULT inserted =
            insert->InsertTextAtSelection(ec, TF_IAS_QUERYONLY, nullptr, 0, &range);
        insert->Release();
        if (FAILED(inserted) || range == nullptr) {
            return inserted;
        }

        ITfContextComposition *composition_host = nullptr;
        if (SUCCEEDED(context->QueryInterface(IID_ITfContextComposition,
                                              reinterpret_cast<void **>(&composition_host))) &&
            composition_host != nullptr) {
            const HRESULT started = composition_host->StartComposition(
                ec, range, static_cast<ITfCompositionSink *>(this), &composition_);
            composition_host->Release();
            if (FAILED(started)) {
                range->Release();
                return started;
            }
            if (composition_context_ != nullptr) {
                composition_context_->Release();
            }
            composition_context_ = context;
            composition_context_->AddRef();
        } else {
            range->Release();
            return E_NOINTERFACE;
        }
        range->Release();
    }

    if (composition_ == nullptr) {
        return E_FAIL;
    }

    ITfRange *range = nullptr;
    if (FAILED(composition_->GetRange(&range)) || range == nullptr) {
        return E_FAIL;
    }
    const HRESULT written =
        range->SetText(ec, 0, text.c_str(), static_cast<LONG>(text.size()));
    if (SUCCEEDED(written)) {
        /* The underline that says "not committed yet": the attribute property
         * holds the atom of the display attribute this service publishes. */
        ITfProperty *attribute_property = nullptr;
        if (SUCCEEDED(context->GetProperty(GUID_PROP_ATTRIBUTE, &attribute_property)) &&
            attribute_property != nullptr) {
            VARIANT value;
            VariantInit(&value);
            value.vt = VT_I4;
            value.lVal = static_cast<LONG>(display_attribute_atom());
            attribute_property->SetValue(ec, range, &value);
            attribute_property->Release();
        }
    }
    range->Release();
    if (FAILED(written)) {
        return written;
    }

    if (finish) {
        ITfComposition *ending = composition_;
        composition_ = nullptr;
        ending->EndComposition(ec);
        ending->Release();
    }
    return S_OK;
}

/* ------------------------------------------------------ display attributes */

STDMETHODIMP TextService::EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo **enum_info) {
    if (enum_info == nullptr) {
        return E_INVALIDARG;
    }
    *enum_info = new (std::nothrow) DisplayAttributeEnumerator();
    return *enum_info != nullptr ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP TextService::GetDisplayAttributeInfo(REFGUID guid, ITfDisplayAttributeInfo **info) {
    if (info == nullptr) {
        return E_INVALIDARG;
    }
    *info = nullptr;
    if (!IsEqualGUID(guid, GUID_RustMagicDisplayAttribute)) {
        return E_INVALIDARG;
    }
    *info = new (std::nothrow) DisplayAttributeInfo();
    return *info != nullptr ? S_OK : E_OUTOFMEMORY;
}

} // namespace rustmagic

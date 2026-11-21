/*
 * text_service.h — the TSF text service: the object Windows calls when a key is
 * pressed while RustMagic is the selected input method.
 *
 * It implements the interfaces TSF requires of a keyboard text service:
 *
 *   ITfTextInputProcessorEx   activation and deactivation
 *   ITfThreadMgrEventSink     focus changes, so a composition cannot leak into
 *                             the next window
 *   ITfKeyEventSink           the keys themselves
 *   ITfCompositionSink        the application ending our composition for us
 *   ITfDisplayAttributeProvider  the underline under composing text
 *
 * The service keeps one composition open while a syllable is being typed. The
 * engine decides what the text should read; the composition is where that text
 * lives until a key ends the syllable, at which point it becomes ordinary text
 * in the application.
 */

#ifndef RUSTMAGIC_TEXT_SERVICE_H
#define RUSTMAGIC_TEXT_SERVICE_H

#include <windows.h>
#include <msctf.h>
#include <string>

#include "engine.h"
#include "tsf_compat.h"

namespace rustmagic {

class TextService : public ITfTextInputProcessorEx,
                    public ITfThreadMgrEventSink,
                    public ITfKeyEventSink,
                    public ITfCompositionSink,
                    public ITfDisplayAttributeProvider {
public:
    TextService();

    /* IUnknown */
    STDMETHODIMP QueryInterface(REFIID riid, void **ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    /* ITfTextInputProcessor / ITfTextInputProcessorEx */
    STDMETHODIMP Activate(ITfThreadMgr *thread_mgr, TfClientId client_id) override;
    STDMETHODIMP Deactivate() override;
    STDMETHODIMP ActivateEx(ITfThreadMgr *thread_mgr, TfClientId client_id, DWORD flags) override;

    /* ITfThreadMgrEventSink */
    STDMETHODIMP OnInitDocumentMgr(ITfDocumentMgr *document_mgr) override;
    STDMETHODIMP OnUninitDocumentMgr(ITfDocumentMgr *document_mgr) override;
    STDMETHODIMP OnSetFocus(ITfDocumentMgr *focused, ITfDocumentMgr *previous) override;
    STDMETHODIMP OnPushContext(ITfContext *context) override;
    STDMETHODIMP OnPopContext(ITfContext *context) override;

    /* ITfKeyEventSink */
    STDMETHODIMP OnSetFocus(BOOL foreground) override;
    STDMETHODIMP OnTestKeyDown(ITfContext *context, WPARAM wparam, LPARAM lparam,
                               BOOL *eaten) override;
    STDMETHODIMP OnTestKeyUp(ITfContext *context, WPARAM wparam, LPARAM lparam, BOOL *eaten) override;
    STDMETHODIMP OnKeyDown(ITfContext *context, WPARAM wparam, LPARAM lparam, BOOL *eaten) override;
    STDMETHODIMP OnKeyUp(ITfContext *context, WPARAM wparam, LPARAM lparam, BOOL *eaten) override;
    STDMETHODIMP OnPreservedKey(ITfContext *context, REFGUID guid, BOOL *eaten) override;

    /* ITfCompositionSink */
    STDMETHODIMP OnCompositionTerminated(TfEditCookie ec, ITfComposition *composition) override;

    /* ITfDisplayAttributeProvider */
    STDMETHODIMP EnumDisplayAttributeInfo(IEnumTfDisplayAttributeInfo **enum_info) override;
    STDMETHODIMP GetDisplayAttributeInfo(REFGUID guid, ITfDisplayAttributeInfo **info) override;

    /* The body of an edit session: writes `text` into the composition of
     * `context`, starting it when there is none and ending it when `finish`. */
    STDMETHODIMP write_composition(ITfContext *context, TfEditCookie ec, const std::wstring &text,
                                   bool finish);

    /* Ends the composition in the current context, keeping the text. Called on
     * focus loss and on deactivation, where leaving a composition open would
     * strand the underline in a window we no longer own. */
    void commit_and_reset(ITfContext *context, bool commit);

private:
    ~TextService() = default;

    /* Applies one engine edit to the composition, then renders it. */
    void apply(ITfContext *context, const Edit &edit, bool finish);
    /* Renders `composing_` into the composition of `context`. */
    void update_composition(ITfContext *context, bool finish);
    /* Releases the composition objects after a failed write or a commit. */
    void finish_cleanup();

    LONG refs_ = 1;

    ITfThreadMgr *thread_mgr_ = nullptr;
    TfClientId client_id_ = TF_CLIENTID_NULL;
    DWORD thread_mgr_cookie_ = TF_INVALID_COOKIE;

    Engine engine_;

    /* The text of the syllable being typed, which is exactly the composition. */
    std::wstring composing_;
    /* The context the composition belongs to, held so a commit can reach it. */
    ITfContext *composition_context_ = nullptr;
    ITfComposition *composition_ = nullptr;
};

} // namespace rustmagic

#endif /* RUSTMAGIC_TEXT_SERVICE_H */

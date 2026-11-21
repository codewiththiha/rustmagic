/*
 * tsf_compat.h — the TSF declarations the mingw headers do not carry.
 *
 * mingw-w64's msctf.h is incomplete: it leaves out ITfDisplayAttributeProvider
 * and ITfTextInputProcessorEx, along with a few constants. The Windows SDK has
 * them, so when building with the SDK these blocks compile away; on a Linux host
 * with mingw-w64 they supply the same declarations with the same identifiers,
 * taken from the published interface definitions (msctf.idl), so the DLL can be
 * compiled and syntax-checked without a Windows machine.
 */

#ifndef RUSTMAGIC_TSF_COMPAT_H
#define RUSTMAGIC_TSF_COMPAT_H

#include <windows.h>
#include <msctf.h>

/* "No client id": what a text service reports before TSF gives it one. The SDK
 * spells it in msctf.idl as ((TfClientId)0). */
#ifndef TF_CLIENTID_NULL
#define TF_CLIENTID_NULL ((TfClientId)0)
#endif

/* {fee47777-163c-4769-996a-6e9c50ad8f54} — implemented by a text service that
 * wants to draw its composing text differently from committed text. */
#ifndef __ITfDisplayAttributeProvider_INTERFACE_DEFINED__
#define __ITfDisplayAttributeProvider_INTERFACE_DEFINED__
EXTERN_C const IID IID_ITfDisplayAttributeProvider;

struct ITfDisplayAttributeProvider : public IUnknown {
    virtual HRESULT STDMETHODCALLTYPE EnumDisplayAttributeInfo(
        IEnumTfDisplayAttributeInfo **enum_info) = 0;
    virtual HRESULT STDMETHODCALLTYPE GetDisplayAttributeInfo(
        REFGUID guid, ITfDisplayAttributeInfo **info) = 0;
};
#endif /* __ITfDisplayAttributeProvider_INTERFACE_DEFINED__ */

/* {6e4e2102-f9cd-433d-b496-303ce03a6507} — the second revision of activation,
 * which tells the service whether it is being activated for the first time in
 * this thread. TSF falls back to Activate when a service does not have it. */
#ifndef __ITfTextInputProcessorEx_INTERFACE_DEFINED__
#define __ITfTextInputProcessorEx_INTERFACE_DEFINED__
EXTERN_C const IID IID_ITfTextInputProcessorEx;

struct ITfTextInputProcessorEx : public ITfTextInputProcessor {
    virtual HRESULT STDMETHODCALLTYPE ActivateEx(ITfThreadMgr *thread_mgr, TfClientId client_id,
                                                 DWORD flags) = 0;
};
#endif /* __ITfTextInputProcessorEx_INTERFACE_DEFINED__ */

#endif /* RUSTMAGIC_TSF_COMPAT_H */

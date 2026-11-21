#include "class_factory.h"

#include <new>

#include "rustmagic_ids.h"
#include "text_service.h"

namespace rustmagic {
namespace {

LONG g_live_objects = 0;
LONG g_server_locks = 0;

} // namespace

LONG note_object_created() { return InterlockedIncrement(&g_live_objects); }

LONG note_object_destroyed() { return InterlockedDecrement(&g_live_objects); }

LONG ClassFactory::live_objects() { return g_live_objects + g_server_locks; }

ClassFactory::ClassFactory() { note_object_created(); }

STDMETHODIMP ClassFactory::QueryInterface(REFIID riid, void **ppv) {
    if (ppv == nullptr) {
        return E_INVALIDARG;
    }
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IClassFactory)) {
        *ppv = static_cast<IClassFactory *>(this);
    } else {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) ClassFactory::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) ClassFactory::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (refs == 0) {
        note_object_destroyed();
        delete this;
    }
    return refs;
}

STDMETHODIMP ClassFactory::CreateInstance(IUnknown *outer, REFIID riid, void **ppv) {
    if (ppv == nullptr) {
        return E_INVALIDARG;
    }
    *ppv = nullptr;
    if (outer != nullptr) {
        return CLASS_E_NOAGGREGATION; /* a text service is never aggregated */
    }
    TextService *service = new (std::nothrow) TextService();
    if (service == nullptr) {
        return E_OUTOFMEMORY;
    }
    const HRESULT result = service->QueryInterface(riid, ppv);
    service->Release();
    return result;
}

STDMETHODIMP ClassFactory::LockServer(BOOL lock) {
    if (lock) {
        InterlockedIncrement(&g_server_locks);
    } else {
        InterlockedDecrement(&g_server_locks);
    }
    return S_OK;
}

} // namespace rustmagic

/*
 * class_factory.h — the COM class factory TSF asks for by CLSID.
 */

#ifndef RUSTMAGIC_CLASS_FACTORY_H
#define RUSTMAGIC_CLASS_FACTORY_H

#include <windows.h>
#include <msctf.h>
#include <unknwn.h>

namespace rustmagic {

/* COM hands out text services through this, never through the DLL directly:
 * the thread manager calls DllGetClassObject, then CreateInstance, and owns the
 * resulting ITfTextInputProcessor for as long as the input method is selected. */
class ClassFactory : public IClassFactory {
public:
    ClassFactory();

    STDMETHODIMP QueryInterface(REFIID riid, void **ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    STDMETHODIMP CreateInstance(IUnknown *outer, REFIID riid, void **ppv) override;
    STDMETHODIMP LockServer(BOOL lock) override;

    /* How many text services and factories are alive; DllCanUnloadNow reads it. */
    static LONG live_objects();

private:
    ~ClassFactory() = default;
    LONG refs_ = 1;
};

/* Counts every TextService and ClassFactory, so the DLL is never freed while
 * an application still holds the input method. */
LONG note_object_created();
LONG note_object_destroyed();

} // namespace rustmagic

#endif /* RUSTMAGIC_CLASS_FACTORY_H */

/*
 * tsf-com.cpp — drives the Windows text service without Windows.
 *
 *   tsf-com [--register] [--unregister] <path to RustMagicTSF.dll>
 *
 * tools/smoke/tsf-com.sh builds the DLL with mingw-w64 and runs this under Wine.
 * Wine is a Windows API implementation, not a stand-in: the DLL is loaded the way
 * an application loads it, registered the way regsvr32 registers it, and asked
 * questions the way the TSF manager asks them.
 *
 * It checks, in order:
 *
 *   - the four exports COM looks for exist and the DLL is loadable;
 *   - the class factory refuses another CLSID and creates the service;
 *   - the service answers to the published identifiers for a text input
 *     processor, key event sink, thread manager event sink, composition sink and
 *     display attribute provider, and to nothing else;
 *   - references are counted, so the DLL cannot be unloaded underneath a live
 *     service;
 *   - DllRegisterServer writes a CLSID that COM can create the service from,
 *     registers a language profile that the profile store lists back, registers
 *     the service in the keyboard category, and leaves none of it behind after
 *     DllUnregisterServer.
 *
 * What it cannot check is a keystroke landing in a document: that needs a real
 * TSF manager, a focused window and a message loop. The Windows CI job covers
 * activation inside a live thread manager, and INSTALL.txt lists the rest as
 * human checks.
 */

#include <windows.h>
#include <msctf.h>
#include <objbase.h>

#include <cstdio>
#include <cstring>

namespace {

using get_class_object_fn = HRESULT(WINAPI *)(REFCLSID, REFIID, void **);
using simple_fn = HRESULT(WINAPI *)(void);

/* RustMagic's identity, written out here rather than imported: the test fails
 * if the service and the test ever disagree about who it is. */
const CLSID kService = {0x6f1c4a2e, 0x9b31, 0x4c8a, {0x9e, 0x1f, 0x2b, 0x7d, 0x5a, 0x0c, 0x3a, 0x11}};
const GUID kProfile = {0xb7e22c60, 0x5d0a, 0x4e4f, {0x8a, 0x2b, 0x1c, 0x3d, 0x9e, 0x7f, 0x4a, 0x55}};

/* The published identifiers, from msctf.idl, and the category TSF uses to mean
 * "this is a keyboard input method". */
const GUID kClassFactory = {0x00000001, 0x0000, 0x0000, {0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46}};
const GUID kTextInputProcessor = {0xaa80e7f7, 0x2021, 0x11d2, {0x93, 0xe0, 0x00, 0x60, 0xb0, 0x67, 0xb8, 0x6e}};
const GUID kKeyEventSink = {0xaa80e7f5, 0x2021, 0x11d2, {0x93, 0xe0, 0x00, 0x60, 0xb0, 0x67, 0xb8, 0x6e}};
const GUID kThreadMgrEventSink = {0xaa80e80e, 0x2021, 0x11d2, {0x93, 0xe0, 0x00, 0x60, 0xb0, 0x67, 0xb8, 0x6e}};
const GUID kCompositionSink = {0xa781718c, 0x579a, 0x4b15, {0xa2, 0x80, 0x32, 0xb8, 0x57, 0x7a, 0xcc, 0x5e}};
const GUID kDisplayAttributeProvider = {0xfee47777, 0x163c, 0x4769, {0x99, 0x6a, 0x6e, 0x9c, 0x50, 0xad, 0x8f, 0x54}};
const GUID kKeyboardCategory = {0x34745c63, 0xb2f0, 0x4784, {0x8b, 0x67, 0x5e, 0x12, 0xc8, 0x70, 0x1a, 0x31}};
const CLSID kProfilesClsid = {0x33c53a50, 0xf456, 0x4884, {0xb0, 0x49, 0x85, 0xfd, 0x64, 0x3e, 0xcf, 0xed}};
const IID kProfilesIid = {0x1f02b6c5, 0x7842, 0x4ee6, {0x8a, 0x0b, 0x9a, 0x24, 0x18, 0x3a, 0x95, 0xca}};
const CLSID kThreadMgrClsid = {0x529a9e6b, 0x6587, 0x4f23, {0xab, 0x9e, 0x9c, 0x7d, 0x68, 0x3e, 0x3c, 0x50}};
const CLSID kCategoryMgrClsid = {0xa4b544a1, 0x438d, 0x4b41, {0x93, 0x25, 0x86, 0x95, 0x23, 0xe2, 0xd6, 0xc7}};
const IID kCategoryMgrIid = {0xc3acefb5, 0xf69d, 0x4905, {0x93, 0x8f, 0xfc, 0xad, 0xcf, 0x4b, 0xe8, 0x30}};

int failures = 0;

void check(const char *what, bool ok) {
    std::printf("%s  %s\n", ok ? "PASS" : "FAIL", what);
    if (!ok) {
        ++failures;
    }
}

/* GetProcAddress hands back a bare FARPROC; going through void* keeps the
 * compiler from warning about incompatible function types. */
template <typename Type>
Type symbol(HMODULE dll, const char *name) {
    return reinterpret_cast<Type>(reinterpret_cast<void *>(GetProcAddress(dll, name)));
}

bool asks_for(IUnknown *object, REFIID interface_id) {
    void *answer = nullptr;
    if (SUCCEEDED(object->QueryInterface(interface_id, &answer)) && answer != nullptr) {
        static_cast<IUnknown *>(answer)->Release();
        return true;
    }
    return false;
}

bool clsid_registered() {
    HKEY key = nullptr;
    const char *path = "CLSID\\{6F1C4A2E-9B31-4C8A-9E1F-2B7D5A0C3A11}\\InprocServer32";
    if (RegOpenKeyExA(HKEY_CLASSES_ROOT, path, 0, KEY_READ, &key) != ERROR_SUCCESS) {
        return false;
    }
    RegCloseKey(key);
    return true;
}

/* Activation is where a text service advises the keystroke manager and the
 * thread manager that it wants to hear from them. Getting that wrong is how a
 * service either never sees a key or crashes on the first focus change, so it is
 * worth driving even where there is no document to type into. */
void activation(IClassFactory *factory) {
    ITfThreadMgr *thread_mgr = nullptr;
    HRESULT hr = CoCreateInstance(kThreadMgrClsid, nullptr, CLSCTX_INPROC_SERVER, IID_ITfThreadMgr,
                                  reinterpret_cast<void **>(&thread_mgr));
    if (FAILED(hr) || thread_mgr == nullptr) {
        std::printf("SKIP  a TSF thread manager is available (this host has none)\n");
        return;
    }
    check("a TSF thread manager is available", true);

    TfClientId client_id = 0;
    hr = thread_mgr->Activate(&client_id);
    check("the thread manager activates this thread", SUCCEEDED(hr) && client_id != 0);

    IUnknown *service = nullptr;
    hr = factory->CreateInstance(nullptr, IID_IUnknown, reinterpret_cast<void **>(&service));
    if (SUCCEEDED(hr) && service != nullptr) {
        ITfTextInputProcessor *processor = nullptr;
        if (SUCCEEDED(service->QueryInterface(kTextInputProcessor,
                                             reinterpret_cast<void **>(&processor))) &&
            processor != nullptr) {
            hr = processor->Activate(thread_mgr, client_id);
            check("the service activates inside the thread manager", SUCCEEDED(hr));
            if (SUCCEEDED(hr)) {
                ITfKeyEventSink *keys = nullptr;
                if (SUCCEEDED(service->QueryInterface(kKeyEventSink,
                                                      reinterpret_cast<void **>(&keys))) &&
                    keys != nullptr) {
                    /* The framework asks this before it asks for the key. With
                     * no keyboard installed nothing may be claimed, or the
                     * service swallows every keystroke in every application. */
                    BOOL eaten = TRUE;
                    keys->OnTestKeyDown(nullptr, 'A', 0, &eaten);
                    check("a test key down with no text field is not claimed", eaten == FALSE);

                    /* Shift is never claimed: it is a modifier, not text. */
                    eaten = TRUE;
                    keys->OnTestKeyDown(nullptr, VK_SHIFT, 0, &eaten);
                    check("a modifier is never claimed", eaten == FALSE);
                    keys->OnSetFocus(TRUE);
                    keys->Release();
                }
                hr = processor->Deactivate();
                check("the service deactivates", SUCCEEDED(hr));
            }
            processor->Release();
        }
        service->Release();
    }
    thread_mgr->Deactivate();
    thread_mgr->Release();
}

/* Is the profile in the store, for a language it was registered under? */
bool profile_registered() {
    ITfInputProcessorProfiles *profiles = nullptr;
    if (FAILED(CoCreateInstance(kProfilesClsid, nullptr, CLSCTX_INPROC_SERVER, kProfilesIid,
                                reinterpret_cast<void **>(&profiles))) ||
        profiles == nullptr) {
        return false;
    }
    const LANGID languages[] = {0x0455, 0x0409};
    bool found = false;
    for (LANGID language : languages) {
        IEnumTfLanguageProfiles *list = nullptr;
        if (FAILED(profiles->EnumLanguageProfiles(language, &list)) || list == nullptr) {
            continue;
        }
        TF_LANGUAGEPROFILE entry = {};
        ULONG fetched = 0;
        while (list->Next(1, &entry, &fetched) == S_OK && fetched == 1) {
            if (IsEqualCLSID(entry.clsid, kService) && IsEqualGUID(entry.guidProfile, kProfile)) {
                found = true;
                break;
            }
        }
        list->Release();
        if (found) {
            break;
        }
    }
    profiles->Release();
    return found;
}

/* Is the service in the keyboard category? That is the entry Settings and the
 * language bar read. A host whose category manager cannot enumerate (Wine's
 * cannot) is reported as unknown rather than counted as a failure. */
enum class Presence { yes, no, unknown };

Presence in_keyboard_category() {
    ITfCategoryMgr *categories = nullptr;
    if (FAILED(CoCreateInstance(kCategoryMgrClsid, nullptr, CLSCTX_INPROC_SERVER, kCategoryMgrIid,
                                reinterpret_cast<void **>(&categories))) ||
        categories == nullptr) {
        return Presence::unknown;
    }
    IEnumGUID *items = nullptr;
    Presence result = Presence::unknown;
    const HRESULT enumerated = categories->EnumItemsInCategory(kKeyboardCategory, &items);
    if (SUCCEEDED(enumerated) && items != nullptr) {
        result = Presence::no;
        GUID item = {};
        ULONG fetched = 0;
        while (items->Next(1, &item, &fetched) == S_OK && fetched == 1) {
            if (IsEqualCLSID(item, kService)) {
                result = Presence::yes;
                break;
            }
        }
        items->Release();
    }
    categories->Release();
    return result;
}

} // namespace

int main(int argc, char **argv) {
    bool do_register = false;
    bool do_unregister = false;
    const char *path = nullptr;

    for (int index = 1; index < argc; ++index) {
        if (std::strcmp(argv[index], "--register") == 0) {
            do_register = true;
        } else if (std::strcmp(argv[index], "--unregister") == 0) {
            do_unregister = true;
        } else {
            path = argv[index];
        }
    }
    if (path == nullptr) {
        std::printf("usage: tsf-com [--register|--unregister] <RustMagicTSF.dll>\n");
        return 2;
    }

    std::printf("RustMagic text service, COM surface\n");
    /* Registration and CoCreateInstance both need COM; regsvr32 would have
     * initialised it before calling DllRegisterServer. */
    const HRESULT com = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    check("COM initialises", SUCCEEDED(com));

    HMODULE dll = LoadLibraryA(path);
    check("the DLL loads", dll != nullptr);
    if (dll == nullptr) {
        return 1;
    }

    auto get_class_object = symbol<get_class_object_fn>(dll, "DllGetClassObject");
    auto can_unload = symbol<simple_fn>(dll, "DllCanUnloadNow");
    auto register_server = symbol<simple_fn>(dll, "DllRegisterServer");
    auto unregister_server = symbol<simple_fn>(dll, "DllUnregisterServer");
    check("DllGetClassObject is exported", get_class_object != nullptr);
    check("DllCanUnloadNow is exported", can_unload != nullptr);
    check("DllRegisterServer is exported", register_server != nullptr);
    check("DllUnregisterServer is exported", unregister_server != nullptr);
    if (get_class_object == nullptr || can_unload == nullptr) {
        return 1;
    }

    IClassFactory *factory = nullptr;
    HRESULT hr = get_class_object(kService, kClassFactory, reinterpret_cast<void **>(&factory));
    check("the class factory is created", SUCCEEDED(hr) && factory != nullptr);
    if (factory == nullptr) {
        return 1;
    }

    /* An unknown CLSID has to be refused rather than crash: applications ask. */
    CLSID other = kService;
    other.Data1 ^= 0xFF;
    IClassFactory *nothing = nullptr;
    hr = get_class_object(other, kClassFactory, reinterpret_cast<void **>(&nothing));
    check("another CLSID is refused", FAILED(hr) && nothing == nullptr);

    IUnknown *service = nullptr;
    hr = factory->CreateInstance(nullptr, IID_IUnknown, reinterpret_cast<void **>(&service));
    check("the service is created", SUCCEEDED(hr) && service != nullptr);
    if (service != nullptr) {
        check("it is a text input processor", asks_for(service, kTextInputProcessor));
        check("it is a key event sink", asks_for(service, kKeyEventSink));
        check("it is a thread manager event sink", asks_for(service, kThreadMgrEventSink));
        check("it is a composition sink", asks_for(service, kCompositionSink));
        check("it publishes display attributes", asks_for(service, kDisplayAttributeProvider));
        check("it does not answer for everything", !asks_for(service, kClassFactory));
        check("the DLL is not unloadable while it lives", can_unload() != S_OK);
        service->Release();
    }
    activation(factory);
    factory->Release();
    check("the DLL is unloadable afterwards", can_unload() == S_OK);

    if (do_register && register_server != nullptr) {
        hr = register_server();
        check("the CLSID is registered", clsid_registered());
        check("a language profile is listed", profile_registered());
        const Presence category = in_keyboard_category();
        if (category == Presence::unknown) {
            /* Wine registers the category but cannot enumerate it back; the
             * Windows job checks this against a real category manager. */
            std::printf("SKIP  it is in the keyboard category (this host cannot enumerate)\n");
        } else {
            check("it is in the keyboard category", category == Presence::yes);
        }
        if (FAILED(hr)) {
            std::printf("      DllRegisterServer returned 0x%08lx\n", static_cast<unsigned long>(hr));
        }

        /* And now the way an application gets the service: through COM, by CLSID,
         * from the registry this DLL just wrote. */
        IUnknown *through_com = nullptr;
        hr = CoCreateInstance(kService, nullptr, CLSCTX_INPROC_SERVER, IID_IUnknown,
                              reinterpret_cast<void **>(&through_com));
        check("COM creates it by CLSID", SUCCEEDED(hr) && through_com != nullptr);
        if (through_com != nullptr) {
            through_com->Release();
        }
    }

    if (do_unregister && unregister_server != nullptr) {
        unregister_server();
        check("the CLSID is gone again", !clsid_registered());
        check("the language profile is gone again", !profile_registered());
        check("it left the keyboard category", in_keyboard_category() != Presence::yes);
    }

    FreeLibrary(dll);
    CoUninitialize();
    if (failures == 0) {
        std::printf("the COM surface is sound\n");
        return 0;
    }
    std::printf("%d check(s) failed\n", failures);
    return 1;
}

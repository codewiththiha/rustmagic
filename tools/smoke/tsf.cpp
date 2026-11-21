/*
 * tsf.cpp — proves the Windows text service is installed and alive.
 *
 * Run it on a real Windows machine after regsvr32 has registered the DLL:
 *
 *   regsvr32 /s RustMagicTSF.dll
 *   tsf-smoke.exe C:\path\to\RustMagicTSF.dll
 *
 * It checks the four things that decide whether a text service is usable at
 * all — that Windows can load it, that it is registered as a keyboard for a
 * language, that its class factory hands out the interfaces TSF asks for, and
 * that it activates inside a real TSF thread manager — and exits non-zero with
 * a line naming whatever came first if any of them is missing.
 *
 * What it cannot check is a keystroke landing in a document: that needs a
 * window, a focus, and a user. That part is stated in INSTALL.txt rather than
 * pretended here.
 */

#include <windows.h>
#include <msctf.h>

#include "../../platform/windows/tsf/tsf_compat.h"

#include <cstdio>
#include <string>

/* The service's identity; duplicated here on purpose so the test fails loudly
 * if anyone edits one copy and not the other. */
static const CLSID kServiceClsid = {0x6f1c4a2e, 0x9b31, 0x4c8a,
                                    {0x9e, 0x1f, 0x2b, 0x7d, 0x5a, 0x0c, 0x3a, 0x11}};
static const GUID kProfileGuid = {0xb7e22c60, 0x5d0a, 0x4e4f,
                                  {0x8a, 0x2b, 0x1c, 0x3d, 0x9e, 0x7f, 0x4a, 0x55}};
static const GUID kShowGuid = {0x34745c63, 0xb2f0, 0x4784,
                               {0x8b, 0x67, 0x5e, 0x12, 0xc8, 0x70, 0x1a, 0x31}};

/* ITfDisplayAttributeProvider's IID, which mingw's headers do not carry and the
 * service defines in its own translation unit. */
static const GUID kAttributeProviderIid = {0xfee47777, 0x163c, 0x4769,
                                           {0x99, 0x6a, 0x6e, 0x9c, 0x50, 0xad, 0x8f, 0x54}};

static int failures = 0;

static void pass(const char *what) { std::printf("PASS  %s\n", what); }

static void fail(const char *what, long code) {
    std::printf("FAIL  %s (0x%08lx)\n", what, code);
    ++failures;
}

/* 1. The registry says a CLSID lives in a DLL, and it is the one we were given. */
static void check_registry(const std::wstring &dll) {
    const std::wstring key = L"CLSID\\" + std::wstring(L"{6F1C4A2E-9B31-4C8A-9E1F-2B7D5A0C3A11}") +
                             L"\\InprocServer32";
    wchar_t value[MAX_PATH * 2] = {};
    DWORD size = sizeof(value);
    const LSTATUS status =
        RegGetValueW(HKEY_CLASSES_ROOT, key.c_str(), nullptr, RRF_RT_REG_SZ, nullptr, value, &size);
    if (status != ERROR_SUCCESS) {
        fail("InprocServer32 is registered", status);
        return;
    }
    if (_wcsicmp(value, dll.c_str()) != 0) {
        std::printf("  registered path: %ls\n  expected path:   %ls\n", value, dll.c_str());
        fail("InprocServer32 points at this DLL", E_UNEXPECTED);
        return;
    }
    pass("InprocServer32 points at this DLL");
}

/* 2. The keyboard category holds the service, which is what puts it in the
 *    language list and lets an application ask for a keyboard input method. */
static void check_keyboard_category() {
    ITfCategoryMgr *categories = nullptr;
    HRESULT hr = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_ITfCategoryMgr, reinterpret_cast<void **>(&categories));
    if (FAILED(hr) || categories == nullptr) {
        fail("the category manager is available", hr);
        return;
    }
    IEnumGUID *items = nullptr;
    hr = categories->EnumItemsInCategory(kShowGuid, &items);
    if (FAILED(hr) || items == nullptr) {
        fail("GUID_TFCAT_TIP_KEYBOARD can be enumerated", hr);
        categories->Release();
        return;
    }
    bool found = false;
    GUID item{};
    ULONG fetched = 0;
    while (items->Next(1, &item, &fetched) == S_OK && fetched == 1) {
        if (IsEqualCLSID(item, kServiceClsid)) {
            found = true;
            break;
        }
    }
    items->Release();
    categories->Release();
    if (found) {
        pass("the service is registered under GUID_TFCAT_TIP_KEYBOARD");
    } else {
        fail("the service is registered under GUID_TFCAT_TIP_KEYBOARD", E_NOTIMPL);
    }
}

/* 3. A language profile exists, which is the entry the user picks in Settings
 *    and the language bar. Without one the service exists but is unselectable. */
static void check_language_profile() {
    ITfInputProcessorProfiles *profiles = nullptr;
    HRESULT hr = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                  IID_ITfInputProcessorProfiles,
                                  reinterpret_cast<void **>(&profiles));
    if (FAILED(hr) || profiles == nullptr) {
        fail("the input processor profiles store is available", hr);
        return;
    }
    const LANGID languages[] = {0x0455, 0x0409};
    bool found = false;
    for (LANGID language : languages) {
        IEnumTfLanguageProfiles *list = nullptr;
        if (FAILED(profiles->EnumLanguageProfiles(language, &list)) || list == nullptr) {
            continue;
        }
        TF_LANGUAGEPROFILE profile{};
        ULONG fetched = 0;
        while (list->Next(1, &profile, &fetched) == S_OK && fetched == 1) {
            if (IsEqualCLSID(profile.clsid, kServiceClsid) &&
                IsEqualGUID(profile.guidProfile, kProfileGuid)) {
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
    if (found) {
        pass("a language profile exists for RustMagic");
    } else {
        fail("a language profile exists for RustMagic", E_NOTIMPL);
    }
}

/* 4. The class factory hands out what TSF needs, and the service activates and
 *    deactivates against a live thread manager. */
static void check_activation() {
    IUnknown *unknown = nullptr;
    HRESULT hr = CoCreateInstance(kServiceClsid, nullptr, CLSCTX_INPROC_SERVER, IID_IUnknown,
                                  reinterpret_cast<void **>(&unknown));
    if (FAILED(hr) || unknown == nullptr) {
        fail("the class factory creates the service", hr);
        return;
    }
    pass("the class factory creates the service");

    ITfTextInputProcessor *processor = nullptr;
    hr = unknown->QueryInterface(IID_ITfTextInputProcessor,
                                 reinterpret_cast<void **>(&processor));
    if (FAILED(hr) || processor == nullptr) {
        fail("the service implements ITfTextInputProcessor", hr);
        unknown->Release();
        return;
    }
    pass("the service implements ITfTextInputProcessor");

    ITfKeyEventSink *keys = nullptr;
    hr = unknown->QueryInterface(IID_ITfKeyEventSink, reinterpret_cast<void **>(&keys));
    if (SUCCEEDED(hr) && keys != nullptr) {
        pass("the service implements ITfKeyEventSink");
        keys->Release();
    } else {
        fail("the service implements ITfKeyEventSink", hr);
    }

    ITfDisplayAttributeProvider *attributes = nullptr;
    hr = unknown->QueryInterface(kAttributeProviderIid,
                                 reinterpret_cast<void **>(&attributes));
    if (SUCCEEDED(hr) && attributes != nullptr) {
        pass("the service implements ITfDisplayAttributeProvider");
        attributes->Release();
    } else {
        fail("the service implements ITfDisplayAttributeProvider", hr);
    }

    ITfThreadMgr *thread_mgr = nullptr;
    hr = CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_ITfThreadMgr,
                          reinterpret_cast<void **>(&thread_mgr));
    if (FAILED(hr) || thread_mgr == nullptr) {
        fail("a TSF thread manager exists", hr);
        processor->Release();
        unknown->Release();
        return;
    }
    TfClientId client_id = TF_CLIENTID_NULL;
    hr = thread_mgr->Activate(&client_id);
    if (FAILED(hr) || client_id == TF_CLIENTID_NULL) {
        fail("the thread manager activates this thread", hr);
        thread_mgr->Release();
        processor->Release();
        unknown->Release();
        return;
    }
    pass("the thread manager activates this thread");

    hr = processor->Activate(thread_mgr, client_id);
    if (SUCCEEDED(hr)) {
        pass("the service activates inside the thread manager");
        const HRESULT off = processor->Deactivate();
        if (SUCCEEDED(off)) {
            pass("the service deactivates cleanly");
        } else {
            fail("the service deactivates cleanly", off);
        }
    } else {
        fail("the service activates inside the thread manager", hr);
    }

    thread_mgr->Deactivate();
    thread_mgr->Release();
    processor->Release();
    unknown->Release();
}

int main(int argc, char **argv) {
    if (argc < 2) {
        std::printf("usage: tsf-smoke <path to RustMagicTSF.dll>\n");
        return 2;
    }
    wchar_t dll[MAX_PATH * 2] = {};
    MultiByteToWideChar(CP_ACP, 0, argv[1], -1, dll, ARRAYSIZE(dll));

    std::printf("RustMagic text service smoke test\n");
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(initialized)) {
        std::printf("FAIL  COM initialises (0x%08lx)\n", initialized);
        return 1;
    }

    check_registry(dll);
    check_keyboard_category();
    check_language_profile();
    check_activation();

    CoUninitialize();
    if (failures == 0) {
        std::printf("the text service is installed and activates\n");
        return 0;
    }
    std::printf("%d check(s) failed\n", failures);
    return 1;
}

/*
 * dllmain.cpp — the DLL surface of the text service: the four exports COM and
 * regsvr32 look for, and the registration that puts RustMagic into Windows'
 * list of keyboards.
 *
 * Registration is what makes the service usable system wide. Three things have
 * to happen, and all three are `regsvr32`'s job to trigger:
 *
 *   1. HKCR\CLSID\{our CLSID}\InprocServer32 -> this DLL, Apartment threading.
 *   2. ITfInputProcessorProfiles::Register + a language profile, so the service
 *      exists as a keyboard for a language.
 *   3. ITfCategoryMgr::RegisterCategory, so Windows knows the service is a
 *      keyboard and may run in the immersive shell (Settings, Start, UWP apps).
 */

#include <windows.h>
#include <msctf.h>
#include <olectl.h>

#include <new>
#include <string>

#include "class_factory.h"
#include "rustmagic_ids.h"
#include "text_service.h"
#include "tsf_categories.h"

namespace {

HINSTANCE module_instance = nullptr;

/* regsvr32 hands the DLL its own path; ask Windows rather than trusting argv. */
std::wstring module_path() {
    wchar_t path[MAX_PATH * 2] = {};
    const DWORD length = GetModuleFileNameW(module_instance, path, ARRAYSIZE(path));
    return std::wstring(path, length);
}

bool write_clsid_registration(const std::wstring &dll, bool install) {
    const std::wstring key = L"CLSID\\{6F1C4A2E-9B31-4C8A-9E1F-2B7D5A0C3A11}";
    if (!install) {
        HKEY classes = nullptr;
        if (RegOpenKeyExW(HKEY_CLASSES_ROOT, L"CLSID", 0, KEY_ALL_ACCESS, &classes) !=
            ERROR_SUCCESS) {
            return false;
        }
        const LSTATUS removed = RegDeleteKeyW(classes, L"{6F1C4A2E-9B31-4C8A-9E1F-2B7D5A0C3A11}\\InprocServer32");
        if (removed == ERROR_SUCCESS || removed == ERROR_FILE_NOT_FOUND) {
            RegDeleteKeyW(classes, L"{6F1C4A2E-9B31-4C8A-9E1F-2B7D5A0C3A11}");
        }
        RegCloseKey(classes);
        return true;
    }

    HKEY key_handle = nullptr;
    if (RegCreateKeyExW(HKEY_CLASSES_ROOT, key.c_str(), 0, nullptr, 0, KEY_WRITE, nullptr,
                        &key_handle, nullptr) != ERROR_SUCCESS) {
        return false;
    }
    const wchar_t *description = RUSTMAGIC_DESCRIPTION;
    RegSetValueExW(key_handle, nullptr, 0, REG_SZ, reinterpret_cast<const BYTE *>(description),
                   static_cast<DWORD>((wcslen(description) + 1) * sizeof(wchar_t)));
    RegCloseKey(key_handle);

    HKEY server = nullptr;
    const std::wstring inproc = key + L"\\InprocServer32";
    if (RegCreateKeyExW(HKEY_CLASSES_ROOT, inproc.c_str(), 0, nullptr, 0, KEY_WRITE, nullptr,
                        &server, nullptr) != ERROR_SUCCESS) {
        return false;
    }
    RegSetValueExW(server, nullptr, 0, REG_SZ, reinterpret_cast<const BYTE *>(dll.c_str()),
                   static_cast<DWORD>((dll.size() + 1) * sizeof(wchar_t)));
    const wchar_t *threading = L"Apartment";
    RegSetValueExW(server, L"ThreadingModel", 0, REG_SZ,
                   reinterpret_cast<const BYTE *>(threading),
                   static_cast<DWORD>((wcslen(threading) + 1) * sizeof(wchar_t)));
    RegCloseKey(server);
    return true;
}

/* Everything that needs a live TSF manager rather than the registry. */
HRESULT register_with_tsf() {
    ITfInputProcessorProfiles *profiles = nullptr;
    HRESULT result = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                      IID_ITfInputProcessorProfiles,
                                      reinterpret_cast<void **>(&profiles));
    if (FAILED(result) || profiles == nullptr) {
        return result;
    }
    const std::wstring dll = module_path();
    const std::wstring description = RUSTMAGIC_DESCRIPTION;

    profiles->Register(CLSID_RustMagicTextService);

    /* The profile manager is the modern entry point: it is what Windows 10 and
     * 11 Settings read, and it can register a profile as enabled by default so
     * the service shows up without the user hunting for it. Older builds only
     * have AddLanguageProfile, which is the fallback. */
    ITfInputProcessorProfileMgr *manager = nullptr;
    const bool have_manager =
        SUCCEEDED(profiles->QueryInterface(IID_ITfInputProcessorProfileMgr,
                                           reinterpret_cast<void **>(&manager))) &&
        manager != nullptr;

    for (int index = 0; index < RUSTMAGIC_LANGUAGES_COUNT; ++index) {
        const LANGID language = RUSTMAGIC_LANGUAGES[index];
        if (have_manager) {
            manager->RegisterProfile(CLSID_RustMagicTextService, language, GUID_RustMagicProfile,
                                     description.c_str(), static_cast<ULONG>(description.size()),
                                     dll.c_str(), static_cast<ULONG>(dll.size()), 0, nullptr, 0,
                                     TRUE, 0);
        } else {
            profiles->AddLanguageProfile(CLSID_RustMagicTextService, language, GUID_RustMagicProfile,
                                         description.c_str(),
                                         static_cast<ULONG>(description.size()), dll.c_str(),
                                         static_cast<ULONG>(dll.size()), 0);
        }
    }
    if (manager != nullptr) {
        manager->Release();
    }
    profiles->Release();

    ITfCategoryMgr *categories = nullptr;
    result = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                              IID_ITfCategoryMgr, reinterpret_cast<void **>(&categories));
    if (FAILED(result) || categories == nullptr) {
        return result;
    }
    categories->RegisterCategory(CLSID_RustMagicTextService, GUID_TFCAT_TIP_KEYBOARD,
                                 CLSID_RustMagicTextService);
    categories->RegisterCategory(CLSID_RustMagicTextService, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
                                 CLSID_RustMagicTextService);
    categories->RegisterCategory(CLSID_RustMagicTextService, GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
                                 CLSID_RustMagicTextService);
    categories->RegisterCategory(CLSID_RustMagicTextService, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
                                 GUID_RustMagicDisplayAttribute);
    categories->Release();
    return S_OK;
}

HRESULT unregister_from_tsf() {
    ITfInputProcessorProfiles *profiles = nullptr;
    HRESULT result = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                                      IID_ITfInputProcessorProfiles,
                                      reinterpret_cast<void **>(&profiles));
    if (SUCCEEDED(result) && profiles != nullptr) {
        for (int index = 0; index < RUSTMAGIC_LANGUAGES_COUNT; ++index) {
            profiles->RemoveLanguageProfile(CLSID_RustMagicTextService, RUSTMAGIC_LANGUAGES[index],
                                            GUID_RustMagicProfile);
        }
        profiles->Unregister(CLSID_RustMagicTextService);
        profiles->Release();
    }

    ITfCategoryMgr *categories = nullptr;
    result = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                              IID_ITfCategoryMgr, reinterpret_cast<void **>(&categories));
    if (SUCCEEDED(result) && categories != nullptr) {
        categories->UnregisterCategory(CLSID_RustMagicTextService, GUID_TFCAT_TIP_KEYBOARD,
                                       CLSID_RustMagicTextService);
        categories->UnregisterCategory(CLSID_RustMagicTextService,
                                       GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
                                       CLSID_RustMagicTextService);
        categories->UnregisterCategory(CLSID_RustMagicTextService, GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
                                       CLSID_RustMagicTextService);
        categories->UnregisterCategory(CLSID_RustMagicTextService,
                                       GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
                                       GUID_RustMagicDisplayAttribute);
        categories->Release();
    }
    return S_OK;
}

} // namespace

// ---------------------------------------------------------------- the DLL

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        module_instance = instance;
        /* The service is loaded into every process that types; nothing here
         * needs to hear about threads being created. */
        DisableThreadLibraryCalls(instance);
    }
    return TRUE;
}

extern "C" HRESULT WINAPI DllGetClassObject(REFCLSID clsid, REFIID riid, void **ppv) {
    if (ppv == nullptr) {
        return E_INVALIDARG;
    }
    *ppv = nullptr;
    if (!IsEqualCLSID(clsid, CLSID_RustMagicTextService)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    rustmagic::ClassFactory *factory = new (std::nothrow) rustmagic::ClassFactory();
    if (factory == nullptr) {
        return E_OUTOFMEMORY;
    }
    const HRESULT result = factory->QueryInterface(riid, ppv);
    factory->Release();
    return result;
}

extern "C" HRESULT WINAPI DllCanUnloadNow() {
    return rustmagic::ClassFactory::live_objects() == 0 ? S_OK : S_FALSE;
}

STDAPI DllRegisterServer() {
    const std::wstring dll = module_path();
    if (!write_clsid_registration(dll, true)) {
        return SELFREG_E_CLASS;
    }
    const HRESULT result = register_with_tsf();
    if (FAILED(result)) {
        write_clsid_registration(dll, false);
        return result;
    }
    return S_OK;
}

STDAPI DllUnregisterServer() {
    unregister_from_tsf();
    return write_clsid_registration(module_path(), false) ? S_OK : SELFREG_E_CLASS;
}

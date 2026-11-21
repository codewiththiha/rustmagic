#include "engine.h"

#include <windows.h>
#include <shlobj.h>

#include <algorithm>
#include <vector>

namespace rustmagic {
namespace {

std::string to_utf8(const std::wstring &text) {
    if (text.empty()) {
        return std::string();
    }
    const int size = WideCharToMultiByte(CP_UTF8, 0, text.c_str(), static_cast<int>(text.size()),
                                         nullptr, 0, nullptr, nullptr);
    if (size <= 0) {
        return std::string();
    }
    std::string out(static_cast<size_t>(size), '\0');
    WideCharToMultiByte(CP_UTF8, 0, text.c_str(), static_cast<int>(text.size()), out.data(), size,
                        nullptr, nullptr);
    return out;
}

std::wstring from_utf8(const char *text) {
    if (text == nullptr || *text == '\0') {
        return std::wstring();
    }
    const int size = MultiByteToWideChar(CP_UTF8, 0, text, -1, nullptr, 0);
    if (size <= 1) {
        return std::wstring();
    }
    std::wstring out(static_cast<size_t>(size), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, text, -1, out.data(), size);
    out.pop_back(); // the terminator MultiByteToWideChar counted
    return out;
}

/* The directory this DLL was loaded from. The installer puts the bundled
 * keyboards next to the DLL, so a user who has never opened the RustMagic
 * window still has something to type with. */
std::wstring dll_directory() {
    HMODULE module = nullptr;
    if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                                GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                            reinterpret_cast<LPCWSTR>(&dll_directory), &module)) {
        return std::wstring();
    }
    wchar_t path[MAX_PATH * 2] = {};
    const DWORD length = GetModuleFileNameW(module, path, ARRAYSIZE(path));
    if (length == 0 || length >= ARRAYSIZE(path)) {
        return std::wstring();
    }
    const std::wstring full(path, length);
    const size_t slash = full.find_last_of(L'\\');
    return slash == std::wstring::npos ? std::wstring() : full.substr(0, slash);
}

bool file_exists(const std::wstring &path) {
    DWORD attributes = GetFileAttributesW(path.c_str());
    return attributes != INVALID_FILE_ATTRIBUTES && (attributes & FILE_ATTRIBUTE_DIRECTORY) == 0;
}

std::wstring join(const std::wstring &directory, const std::wstring &name) {
    std::wstring out = directory;
    if (!out.empty() && out.back() != L'\\') {
        out.push_back(L'\\');
    }
    return out + name;
}

/* `active = "my-win"` in settings.toml, without a TOML parser: the RustMagic
 * window writes that file, and this reads the one key it needs. A malformed
 * file is not an error here — the caller falls back to the first keyboard. */
std::wstring active_id_from(const std::wstring &settings_path) {
    HANDLE file = CreateFileW(settings_path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                              OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) {
        return std::wstring();
    }
    LARGE_INTEGER size{};
    if (!GetFileSizeEx(file, &size) || size.QuadPart <= 0 || size.QuadPart > 1 << 20) {
        CloseHandle(file);
        return std::wstring();
    }
    std::string bytes(static_cast<size_t>(size.QuadPart), '\0');
    DWORD read = 0;
    const BOOL ok = ReadFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &read, nullptr);
    CloseHandle(file);
    if (!ok) {
        return std::wstring();
    }
    bytes.resize(read);

    const std::wstring text = from_utf8(bytes.c_str());
    const std::wstring key = L"active";
    size_t position = 0;
    while ((position = text.find(key, position)) != std::wstring::npos) {
        const bool at_line_start = position == 0 || text[position - 1] == L'\n' ||
                                   text[position - 1] == L'\r';
        size_t cursor = position + key.size();
        while (cursor < text.size() && (text[cursor] == L' ' || text[cursor] == L'\t')) {
            ++cursor;
        }
        if (at_line_start && cursor < text.size() && text[cursor] == L'=') {
            ++cursor;
            while (cursor < text.size() && (text[cursor] == L' ' || text[cursor] == L'\t')) {
                ++cursor;
            }
            if (cursor < text.size() && text[cursor] == L'"') {
                const size_t end = text.find(L'"', cursor + 1);
                if (end != std::wstring::npos) {
                    return text.substr(cursor + 1, end - cursor - 1);
                }
            }
            return std::wstring();
        }
        position += key.size();
    }
    return std::wstring();
}

/* The first .km2 in a directory, sorted so the choice is stable across runs. */
std::wstring first_keyboard_in(const std::wstring &directory) {
    WIN32_FIND_DATAW found{};
    const std::wstring pattern = join(directory, L"*.km2");
    HANDLE search = FindFirstFileW(pattern.c_str(), &found);
    if (search == INVALID_HANDLE_VALUE) {
        return std::wstring();
    }
    std::vector<std::wstring> names;
    do {
        if ((found.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) == 0) {
            names.push_back(found.cFileName);
        }
    } while (FindNextFileW(search, &found));
    FindClose(search);
    if (names.empty()) {
        return std::wstring();
    }
    std::sort(names.begin(), names.end());
    return join(directory, names.front());
}

} // namespace

std::wstring state_directory() {
    wchar_t buffer[MAX_PATH * 2] = {};
    const DWORD length = GetEnvironmentVariableW(L"APPDATA", buffer, ARRAYSIZE(buffer));
    if (length == 0 || length >= ARRAYSIZE(buffer)) {
        return L"RustMagic";
    }
    return join(std::wstring(buffer, length), L"RustMagic");
}

std::wstring active_keyboard_path() {
    const std::wstring state = state_directory();
    const std::wstring keyboards = join(state, L"keyboards");
    const std::wstring active = active_id_from(join(state, L"settings.toml"));
    if (!active.empty()) {
        const std::wstring path = join(keyboards, active + L".km2");
        if (file_exists(path)) {
            return path;
        }
    }
    const std::wstring first = first_keyboard_in(keyboards);
    if (!first.empty()) {
        return first;
    }
    /* Next to this DLL: the five layouts the installer ships, so a machine where
     * nobody has opened the window yet still types something. */
    const std::wstring installed = first_keyboard_in(join(dll_directory(), L"keyboards"));
    if (!installed.empty()) {
        return installed;
    }
    /* And the machine-wide copy, for an installer that put them there instead. */
    wchar_t program_data[MAX_PATH * 2] = {};
    const DWORD length =
        GetEnvironmentVariableW(L"ProgramData", program_data, ARRAYSIZE(program_data));
    if (length > 0 && length < ARRAYSIZE(program_data)) {
        const std::wstring shared =
            join(std::wstring(program_data, length), L"RustMagic\\keyboards");
        return first_keyboard_in(shared);
    }
    return std::wstring();
}

size_t character_count(const std::wstring &text) {
    size_t count = 0;
    for (size_t index = 0; index < text.size(); ++index) {
        const wchar_t unit = text[index];
        if (unit >= 0xD800 && unit <= 0xDBFF && index + 1 < text.size()) {
            ++index; // a surrogate pair is one character
        }
        ++count;
    }
    return count;
}

Engine::Engine() = default;

Engine::~Engine() { unload(); }

void Engine::unload() {
    if (engine_ != nullptr) {
        rm_engine_free(engine_);
        engine_ = nullptr;
    }
    if (keyboard_ != nullptr) {
        rm_keyboard_free(keyboard_);
        keyboard_ = nullptr;
    }
    keyboard_id_.clear();
    path_.clear();
}

bool Engine::reload() {
    const std::wstring path = active_keyboard_path();
    if (path.empty()) {
        unload();
        return false;
    }
    if (path == path_ && engine_ != nullptr) {
        return true;
    }
    unload();
    char *error = nullptr;
    const std::string narrow = to_utf8(path);
    rm_keyboard *keyboard = rm_keyboard_load(narrow.c_str(), &error);
    if (keyboard == nullptr) {
        if (error != nullptr) {
            rm_string_free(error);
        }
        return false;
    }
    rm_engine *engine = rm_engine_new(keyboard);
    if (engine == nullptr) {
        rm_keyboard_free(keyboard);
        return false;
    }
    keyboard_ = keyboard;
    engine_ = engine;
    path_ = path;
    const size_t slash = path.find_last_of(L'\\');
    std::wstring name = slash == std::wstring::npos ? path : path.substr(slash + 1);
    const size_t dot = name.find_last_of(L'.');
    keyboard_id_ = dot == std::wstring::npos ? name : name.substr(0, dot);
    return true;
}

void Engine::reset() {
    if (engine_ != nullptr) {
        rm_engine_reset(engine_);
    }
}

Edit Engine::process(unsigned int vk, wchar_t ch, bool shift, bool control, bool alt, bool alt_gr,
                     bool caps) {
    Edit edit;
    if (engine_ == nullptr) {
        return edit;
    }
    rm_edit out{};
    const int consumed = rm_engine_key(engine_, static_cast<uint16_t>(vk), static_cast<uint32_t>(ch),
                                       shift ? 1 : 0, control ? 1 : 0, alt ? 1 : 0,
                                       alt_gr ? 1 : 0, caps ? 1 : 0, &out);
    edit.consumed = consumed != 0;
    edit.erase = out.erase;
    edit.insert = from_utf8(out.insert);
    if (out.insert != nullptr) {
        rm_string_free(out.insert);
    }
    return edit;
}

} // namespace rustmagic

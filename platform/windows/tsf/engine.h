/*
 * engine.h — the TSF service's view of the RustMagic engine.
 *
 * The service owns no rules. It finds the keyboard the RustMagic window has
 * active, hands keys to the engine through the C ABI in rustmagic.h, and gets
 * back an edit: "erase this many characters before the caret, write this".
 * Everything the engine decides is a rule; everything here is plumbing.
 */

#ifndef RUSTMAGIC_ENGINE_H
#define RUSTMAGIC_ENGINE_H

#include <string>

#include "../../../crates/km-ffi/include/rustmagic.h"

namespace rustmagic {

/* What to do to the document after a key. `erase` counts characters, not UTF-16
 * code units: the engine works in characters, and a Myanmar syllable can hold
 * several code units in one character. */
struct Edit {
    size_t erase = 0;
    std::wstring insert;
    bool consumed = false;
};

/* One engine, bound to whichever keyboard is active in the RustMagic window.
 * Not thread safe; TSF calls it from one apartment. */
class Engine {
public:
    Engine();
    ~Engine();

    Engine(const Engine &) = delete;
    Engine &operator=(const Engine &) = delete;

    /* Re-reads the active keyboard from the RustMagic state directory. Cheap
     * unless the path changed, so it is called on every focus change and commit:
     * switching keyboards in the window switches them here. */
    bool reload();

    /* True when a keyboard is loaded and can process keys. */
    bool ready() const { return engine_ != nullptr; }

    /* The id of the loaded keyboard, for the log. */
    const std::wstring &keyboard_id() const { return keyboard_id_; }

    void reset();

    /* Feeds one key. `ch` is the character the key produced, or 0 when it has
     * none. Returns the edit the document needs. */
    Edit process(unsigned int vk, wchar_t ch, bool shift, bool control, bool alt, bool alt_gr,
                 bool caps);

private:
    void unload();

    rm_keyboard *keyboard_ = nullptr;
    rm_engine *engine_ = nullptr;
    std::wstring keyboard_id_;
    std::wstring path_;
};

/* The directory the RustMagic window keeps its state in. */
std::wstring state_directory();

/* The .km2 the service should load: the active keyboard, else the first one the
 * library has, else one from the machine-wide copy an installer places. */
std::wstring active_keyboard_path();

/* The characters in a UTF-8 string, which is how the engine counts erasures. */
size_t character_count(const std::wstring &text);

} // namespace rustmagic

#endif /* RUSTMAGIC_ENGINE_H */

/*
 * rustmagic.h — the C ABI of the RustMagic engine.
 *
 * Platform input-method adapters (a Windows TSF service, a macOS input method,
 * anything that cannot link Rust) use this instead of bindings. The rules of the
 * surface are deliberately small:
 *
 *   * every pointer returned by this library is owned by the caller and must be
 *     released with the matching `_free` function, never with `free`;
 *   * every string is UTF-8 and NUL-terminated;
 *   * a null handle is never dereferenced: calls on it return a neutral answer;
 *   * an error is a message string, not an errno.
 *
 * Link with `-lkm_ffi` (libkm_ffi.so / km_ffi.dll / libkm_ffi.dylib), or build
 * the crate with `crate-type = ["staticlib"]` and link the archive.
 *
 * A complete example lives in tools/smoke/abi.c.
 */

#ifndef RUSTMAGIC_H
#define RUSTMAGIC_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* An opaque loaded keyboard. */
typedef struct rm_keyboard rm_keyboard;
/* An opaque engine driving one composition. */
typedef struct rm_engine rm_engine;

/* What the host should write after a key. */
typedef struct rm_edit {
    /* Characters to delete before the caret. */
    size_t erase;
    /* Text to write at the caret; release with rm_string_free. Never null. */
    char *insert;
    /* 1 when the engine handled the key and the host should not. */
    int consumed;
    /* 1 when matching gave up after the recursion limit. */
    int recursion_limit;
} rm_edit;

/* The library version, as a static string that must not be freed. */
const char *rm_version(void);

/* Loads a `.km2` or `.kms` file. Returns NULL on failure; when `error` is not
 * NULL it receives a message to release with rm_string_free. */
rm_keyboard *rm_keyboard_load(const char *path, char **error);

/* Loads a keyboard from `.km2` bytes already in memory. */
rm_keyboard *rm_keyboard_from_bytes(const uint8_t *bytes, size_t len, char **error);

/* Releases a keyboard. Engines made from it keep working. */
void rm_keyboard_free(rm_keyboard *keyboard);

/* Keyboard metadata. Each returns a string owned by the caller. */
char *rm_keyboard_name(const rm_keyboard *keyboard);
char *rm_keyboard_description(const rm_keyboard *keyboard);
char *rm_keyboard_hotkey(const rm_keyboard *keyboard);

/* How many rules the keyboard has, or 0 for a null handle. */
size_t rm_keyboard_rule_count(const rm_keyboard *keyboard);

/* Creates an engine for a keyboard, and releases it. */
rm_engine *rm_engine_new(const rm_keyboard *keyboard);
void rm_engine_free(rm_engine *engine);

/* Forgets the composition, keeping the rules and switches. */
void rm_engine_reset(rm_engine *engine);

/* Replaces the composition with `text`; the string is copied. */
void rm_engine_set_context(rm_engine *engine, const char *text);

/* The composition so far, owned by the caller. */
char *rm_engine_context(const rm_engine *engine);

/*
 * Feeds one key to the engine.
 *
 *   key     virtual-key code, 0 when the key has none (see km_core::vk)
 *   ch      the character the keyboard produced for the key, or 0
 *   shift, control, alt, alt_gr, caps   modifier state, 0 or 1
 *   out     optional; receives the edit the host should apply
 *
 * Returns 1 when the engine consumed the key, 0 when the host should type it.
 */
int rm_engine_key(rm_engine *engine, uint16_t key, uint32_t ch, int shift, int control,
                  int alt, int alt_gr, int caps, rm_edit *out);

/* Releases a string this library returned. Must not be called twice. */
void rm_string_free(char *text);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RUSTMAGIC_H */

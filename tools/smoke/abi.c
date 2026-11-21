/*
 * Links against the C ABI and types one key, so a broken ABI fails the build
 * rather than a downstream adapter. Run by `cargo xtask abi` and by CI.
 */
#include <stdio.h>
#include <string.h>

#include "rustmagic.h"

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "fixtures/basic.km2";

    char *error = NULL;
    rm_keyboard *keyboard = rm_keyboard_load(path, &error);
    if (keyboard == NULL) {
        fprintf(stderr, "load failed: %s\n", error != NULL ? error : "unknown");
        if (error != NULL) rm_string_free(error);
        return 1;
    }
    char *name = rm_keyboard_name(keyboard);
    char *hotkey = rm_keyboard_hotkey(keyboard);
    printf("rustmagic %s loaded \"%s\" (%zu rules, hotkey \"%s\")\n",
           rm_version(), name, rm_keyboard_rule_count(keyboard), hotkey);
    rm_string_free(name);
    rm_string_free(hotkey);

    rm_engine *engine = rm_engine_new(keyboard);
    if (engine == NULL) {
        fprintf(stderr, "no engine\n");
        rm_keyboard_free(keyboard);
        return 1;
    }

    /* The question a Windows text service asks before it claims a key, and the
     * answer, which must agree with what processing the key then does. */
    int would = rm_engine_key_test(engine, 'A', 'a', 0, 0, 0, 0, 0);
    printf("would key a be taken: %d\n", would);

    rm_edit edit = {0, NULL, 0, 0};
    int consumed = rm_engine_key(engine, 'A', 'a', 0, 0, 0, 0, 0, &edit);
    printf("agrees: %s\n", would == consumed ? "yes" : "no");
    if (would != consumed) return 1;
    printf("key a: consumed=%d erase=%zu insert=\"%s\"\n", consumed, edit.erase,
           edit.insert != NULL ? edit.insert : "");
    if (edit.insert != NULL) rm_string_free(edit.insert);

    char *context = rm_engine_context(engine);
    printf("composition: \"%s\"\n", context);
    rm_string_free(context);

    /* A null handle must be answered, not dereferenced. */
    printf("null rule count: %zu, null context: \"%s\"\n", rm_keyboard_rule_count(NULL),
           (context = rm_engine_context(NULL)));
    rm_string_free(context);

    rm_engine_free(engine);
    rm_keyboard_free(keyboard);
    return 0;
}

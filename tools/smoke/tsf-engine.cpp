/*
 * tsf-engine.cpp — the text service's engine wrapper, on its own.
 *
 *   tsf-engine <state-directory>
 *
 * The service's job is to find a keyboard and to ask the engine two questions
 * about a key: "would you take it?" (TSF asks that first, and a wrong answer
 * either swallows the application's keystrokes or loses the user's) and "what
 * does it do?". This drives `rustmagic::Engine` — the wrapper the service uses,
 * compiled from the same source — against the keyboards in a state directory,
 * under Wine or on Windows, with no TSF in the way.
 *
 * The point is the agreement in the last section: the test pass must answer
 * exactly what the real pass then does, and must leave the engine's state alone,
 * because TSF asks the first question about every single key press.
 *
 * tools/smoke/tsf-com.sh runs it against the bundled layouts.
 */

#include <windows.h>

#include <cstdio>
#include <string>

#include "../../platform/windows/tsf/engine.h"

namespace {

int failures = 0;

void check(const char *what, bool ok) {
    std::printf("%s  %s\n", ok ? "PASS" : "FAIL", what);
    if (!ok) {
        ++failures;
    }
}

/* The keys the agreement check uses: a letter, a digit, a space and a function
 * key, so a layout that claims everything cannot pass by accident. */
struct Sample {
    const char *name;
    unsigned int vk;
    wchar_t ch;
};

const Sample samples[] = {
    {"a", 0x41, L'a'},
    {"m", 0x4D, L'm'},
    {"1", 0x31, L'1'},
    {"space", 0x20, L' '},
    {"F5", 0x74, 0},
    {"left arrow", 0x25, 0},
};

} // namespace

int main(int argc, char **argv) {
    if (argc < 2) {
        std::printf("usage: tsf-engine <state-directory>\n");
        return 2;
    }
    /* The service reads %APPDATA%\RustMagic; pointing APPDATA at a prepared tree
     * is how this runs anywhere. */
    const std::string state = argv[1];
    std::wstring wide(state.begin(), state.end());
    _wputenv_s(L"APPDATA", wide.c_str());

    std::printf("RustMagic text service, engine wrapper\n");

    rustmagic::Engine engine;
    check("a keyboard in the state directory is found and loaded", engine.reload());
    if (!engine.ready()) {
        std::printf("      put the keyboards in %s\\RustMagic\\keyboards and try again\n",
                    argv[1]);
        return 1;
    }
    check("the active keyboard is the one named in settings.toml",
          engine.keyboard_id() == L"pyidaungsu-mm");
    std::printf("      keyboard: %ls\n", engine.keyboard_id().c_str());

    /* The claim decision has to be free: TSF asks it on every key press. */
    bool stable = true;
    for (const Sample &sample : samples) {
        const bool claimed = engine.would_consume(sample.vk, sample.ch, false, false, false, false,
                                                 false);
        (void)claimed;
        if (!engine.ready() || engine.keyboard_id() != L"pyidaungsu-mm") {
            stable = false;
        }
    }
    check("asking does not unload or change the keyboard", stable);

    /* And the answers agree with what processing the key actually does. */
    bool agrees = true;
    for (const Sample &sample : samples) {
        rustmagic::Engine asking;
        rustmagic::Engine doing;
        if (!asking.reload() || !doing.reload()) {
            agrees = false;
            break;
        }
        const bool claimed = asking.would_consume(sample.vk, sample.ch, false, false, false,
                                                  false, false);
        const rustmagic::Edit edit = doing.process(sample.vk, sample.ch, false, false, false,
                                                  false, false);
        if (claimed != edit.consumed) {
            std::printf("      %s: asked %d, did %d\n", sample.name, claimed ? 1 : 0,
                        edit.consumed ? 1 : 0);
            agrees = false;
        }
        /* Asking twice must not drift either. */
        if (asking.would_consume(sample.vk, sample.ch, false, false, false, false, false) !=
            claimed) {
            std::printf("      %s: the answer changed on the second ask\n", sample.name);
            agrees = false;
        }
    }
    check("the test pass agrees with the real pass, key by key", agrees);

    /* A space ends the syllable, so the engine must not claim it: the
     * application inserts it. */
    rustmagic::Engine syllable;
    if (syllable.reload()) {
        const rustmagic::Edit edit = syllable.process(0x20, L' ', false, false, false, false, false);
        check("space is left to the application", !edit.consumed);
    }

    if (failures == 0) {
        std::printf("the engine wrapper is sound\n");
        return 0;
    }
    std::printf("%d check(s) failed\n", failures);
    return 1;
}

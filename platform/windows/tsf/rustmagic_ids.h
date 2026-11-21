/*
 * rustmagic_ids.h — the identifiers Windows knows this text service by.
 *
 * These four GUIDs are the service's identity: the CLSID is what the registry
 * maps to this DLL, the profile GUID is what Windows Language settings stores
 * when a user adds RustMagic, and the display attribute GUID is what makes the
 * composing text underlined instead of looking like text that was already
 * typed. Changing any of them orphans every install that exists.
 *
 * The values themselves are defined once, in tsf_guids.cpp, together with the
 * TSF constants mingw only declares.
 */

#ifndef RUSTMAGIC_IDS_H
#define RUSTMAGIC_IDS_H

#include <windows.h>

/* {6F1C4A2E-9B31-4C8A-9E1F-2B7D5A0C3A11} */
DEFINE_GUID(CLSID_RustMagicTextService, 0x6f1c4a2e, 0x9b31, 0x4c8a, 0x9e, 0x1f, 0x2b, 0x7d,
            0x5a, 0x0c, 0x3a, 0x11);

/* {B7E22C60-5D0A-4E4F-8A2B-1C3D9E7F4A55} */
DEFINE_GUID(GUID_RustMagicProfile, 0xb7e22c60, 0x5d0a, 0x4e4f, 0x8a, 0x2b, 0x1c, 0x3d, 0x9e,
            0x7f, 0x4a, 0x55);

/* {3C9A51D7-2E48-4B36-9C0D-7A5B1E8F2D64} */
DEFINE_GUID(GUID_RustMagicDisplayAttribute, 0x3c9a51d7, 0x2e48, 0x4b36, 0x9c, 0x0d, 0x7a, 0x5b,
            0x1e, 0x8f, 0x2d, 0x64);

/* The description Windows shows in Language settings, and the languages the
 * profile is registered under. Burmese first: it is what the keyboards write. */
#define RUSTMAGIC_DESCRIPTION L"RustMagic"
#define RUSTMAGIC_LANGUAGES_COUNT 2
static const LANGID RUSTMAGIC_LANGUAGES[RUSTMAGIC_LANGUAGES_COUNT] = {
    0x0455, /* my-MM, Burmese */
    0x0409, /* en-US, so the layout is reachable from an English session */
};

#endif /* RUSTMAGIC_IDS_H */

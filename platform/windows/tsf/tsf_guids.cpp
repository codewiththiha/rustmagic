/*
 * tsf_guids.cpp — the identifiers the service is known by, defined once.
 *
 * Two kinds live here and they are handled differently on purpose.
 *
 * Ours — the CLSID, the profile GUID and the display attribute GUID — belong to
 * RustMagic and to nothing else, so they are always defined in this file.
 *
 * The platform's — the TSF classes and categories — are defined by uuid.lib when
 * building against the Windows SDK, and only *declared* by mingw-w64's headers.
 * Under MSVC, defining them again is a duplicate symbol at the very last step of
 * the link, so that half is compiled only where the platform does not provide it.
 *
 * For mingw the definitions go through DEFINE_GUID with INITGUID in effect,
 * which marks them selectany; that is what lets them coexist with the copies in
 * libuuid.a instead of colliding with them.
 */

/* INITGUID is what turns the TSF declarations into definitions. It has to be in
 * effect before the headers that declare them, so it comes first. */
#if !defined(_MSC_VER) || defined(__MINGW32__)
#include <initguid.h>
#endif

#include <windows.h>
#include <msctf.h>

#include "rustmagic_ids.h"
#include "tsf_categories.h"

#if defined(_MSC_VER) && !defined(__MINGW32__)

/* Nothing declares these selectany on the SDK, so they are written out by hand
 * here; that is a plain definition of the declaration in rustmagic_ids.h. */

extern "C" const GUID CLSID_RustMagicTextService = {
    0x6f1c4a2e, 0x9b31, 0x4c8a, {0x9e, 0x1f, 0x2b, 0x7d, 0x5a, 0x0c, 0x3a, 0x11}};

extern "C" const GUID GUID_RustMagicProfile = {
    0xb7e22c60, 0x5d0a, 0x4e4f, {0x8a, 0x2b, 0x1c, 0x3d, 0x9e, 0x7f, 0x4a, 0x55}};

extern "C" const GUID GUID_RustMagicDisplayAttribute = {
    0x3c9a51d7, 0x2e48, 0x4b36, {0x9c, 0x0d, 0x7a, 0x5b, 0x1e, 0x8f, 0x2d, 0x64}};

#else

DEFINE_GUID(CLSID_TF_InputProcessorProfiles, 0x33c53a50, 0xf456, 0x4884, 0xb0, 0x49, 0x85, 0xfd,
            0x64, 0x3e, 0xcf, 0xed);

DEFINE_GUID(CLSID_TF_CategoryMgr, 0xa4b544a1, 0x438d, 0x4b41, 0x93, 0x25, 0x86, 0x95, 0x23, 0xe2,
            0xd6, 0xc7);

/* What Windows calls a keyboard input method. */
DEFINE_GUID(GUID_TFCAT_TIP_KEYBOARD, 0x34745c63, 0xb2f0, 0x4784, 0x8b, 0x67, 0x5e, 0x12, 0xc8, 0x70,
            0x1a, 0x31);

DEFINE_GUID(GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER, 0x046b8c80, 0x1647, 0x40f7, 0x9b, 0x21, 0xb9, 0x3b,
            0x81, 0xaa, 0xbc, 0x1b);

/* "This service may run in the immersive shell", and "do not unload it when the
 * system tray goes away". mingw's headers do not declare these at all. */
DEFINE_GUID(GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, 0x13a016df, 0x560b, 0x46cd, 0x94, 0x7a, 0x4c, 0x3a,
            0xf1, 0xe0, 0xe3, 0x5d);

DEFINE_GUID(GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, 0x25504fb4, 0x7bab, 0x4bc1, 0x9c, 0x69, 0xcf, 0x81,
            0x89, 0x0f, 0x0e, 0xf5);

/* The two interfaces mingw leaves out, which tsf_compat.h declares. */
DEFINE_GUID(IID_ITfDisplayAttributeProvider, 0xfee47777, 0x163c, 0x4769, 0x99, 0x6a, 0x6e, 0x9c,
            0x50, 0xad, 0x8f, 0x54);

DEFINE_GUID(IID_ITfTextInputProcessorEx, 0x6e4e2102, 0xf9cd, 0x433d, 0xb4, 0x96, 0x30, 0x3c, 0xe0,
            0x3a, 0x65, 0x07);

#endif /* the platform does not provide them */

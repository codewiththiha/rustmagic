/*
 * tsf_categories.h — the predefined category GUIDs the mingw headers leave as
 * bare declarations, plus the two capability categories they do not mention.
 *
 * Windows identifies what a text service *is* by the categories it registers
 * under. Getting one of these values wrong is the difference between an input
 * method that appears in Settings and a DLL nobody calls.
 */

#ifndef RUSTMAGIC_TSF_CATEGORIES_H
#define RUSTMAGIC_TSF_CATEGORIES_H

#include <windows.h>
#include <msctf.h>

/* Keyed from MSDN, "Predefined Category Values" and the SDK's msctf.idl. */

/* CLSID_TF_InputProcessorProfiles {33C53A50-F456-4884-B049-85FD643ECFED} */
/* CLSID_TF_CategoryMgr            {A4B544A1-438D-4B41-9325-869523E2D6C7} */
/* GUID_TFCAT_TIP_KEYBOARD         {34745C63-B2F0-4784-8B67-5E12C8701A31} */
/* GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER {046B8C80-1647-40F7-9B21-B93B81AABC1B} */

/* "This service can run in the immersive (Windows Store) shell." Without it the
 * service is not offered to UWP text fields, which is most of the Settings app
 * and every packaged app. */
EXTERN_C const GUID GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT;

/* "Do not kill this service when the system tray becomes unavailable." */
EXTERN_C const GUID GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT;

#endif /* RUSTMAGIC_TSF_CATEGORIES_H */

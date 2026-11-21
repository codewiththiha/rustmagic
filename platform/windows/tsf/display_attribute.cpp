/*
 * display_attribute.cpp — the underline under text that is still composing.
 *
 * TSF needs a provider so that "this text is not committed yet" can be drawn
 * differently. RustMagic uses a plain single underline in the accent colour;
 * the provider is what makes ITfRange::SetGUID(TF_GUID_DISPLAY_ATTRIBUTE) mean
 * something.
 */

#include <windows.h>
#include <msctf.h>

#include <new>

#include "rustmagic_ids.h"
#include "display_attribute.h"

namespace rustmagic {

DisplayAttributeInfo::DisplayAttributeInfo() = default;

STDMETHODIMP DisplayAttributeInfo::QueryInterface(REFIID riid, void **ppv) {
    if (ppv == nullptr) {
        return E_INVALIDARG;
    }
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfDisplayAttributeInfo)) {
        *ppv = static_cast<ITfDisplayAttributeInfo *>(this);
    } else {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) DisplayAttributeInfo::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) DisplayAttributeInfo::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (refs == 0) {
        delete this;
    }
    return refs;
}

STDMETHODIMP DisplayAttributeInfo::GetGUID(GUID *guid) {
    if (guid == nullptr) {
        return E_INVALIDARG;
    }
    *guid = GUID_RustMagicDisplayAttribute;
    return S_OK;
}

STDMETHODIMP DisplayAttributeInfo::GetDescription(BSTR *description) {
    if (description == nullptr) {
        return E_INVALIDARG;
    }
    *description = SysAllocString(L"RustMagic composing text");
    return *description != nullptr ? S_OK : E_OUTOFMEMORY;
}

STDMETHODIMP DisplayAttributeInfo::GetAttributeInfo(TF_DISPLAYATTRIBUTE *attribute) {
    if (attribute == nullptr) {
        return E_INVALIDARG;
    }
    attribute->crText.type = TF_CT_NONE;
    attribute->crText.cr = 0;
    attribute->crBk.type = TF_CT_NONE;
    attribute->crBk.cr = 0;
    attribute->lsStyle = TF_LS_SOLID;
    attribute->fBoldLine = TRUE;
    attribute->crLine.type = TF_CT_COLORREF;
    attribute->crLine.cr = RGB(0x00, 0x78, 0xD7); /* the window's accent */
    attribute->bAttr = TF_ATTR_INPUT;
    return S_OK;
}

STDMETHODIMP DisplayAttributeInfo::SetAttributeInfo(const TF_DISPLAYATTRIBUTE *) {
    return E_NOTIMPL; /* the attribute is fixed */
}

STDMETHODIMP DisplayAttributeInfo::Reset() { return S_OK; }

DisplayAttributeEnumerator::DisplayAttributeEnumerator() {
    info_ = new (std::nothrow) DisplayAttributeInfo();
    if (info_ != nullptr) {
        info_->AddRef();
    }
}

DisplayAttributeEnumerator::~DisplayAttributeEnumerator() {
    if (info_ != nullptr) {
        info_->Release();
    }
}

STDMETHODIMP DisplayAttributeEnumerator::QueryInterface(REFIID riid, void **ppv) {
    if (ppv == nullptr) {
        return E_INVALIDARG;
    }
    *ppv = nullptr;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IEnumTfDisplayAttributeInfo)) {
        *ppv = static_cast<IEnumTfDisplayAttributeInfo *>(this);
    } else {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) DisplayAttributeEnumerator::AddRef() { return InterlockedIncrement(&refs_); }

STDMETHODIMP_(ULONG) DisplayAttributeEnumerator::Release() {
    const LONG refs = InterlockedDecrement(&refs_);
    if (refs == 0) {
        delete this;
    }
    return refs;
}

/* The set holds exactly one attribute, so this is a two state enumerator. */
STDMETHODIMP DisplayAttributeEnumerator::Clone(IEnumTfDisplayAttributeInfo **clone) {
    if (clone == nullptr) {
        return E_INVALIDARG;
    }
    DisplayAttributeEnumerator *copy = new (std::nothrow) DisplayAttributeEnumerator();
    if (copy == nullptr) {
        return E_OUTOFMEMORY;
    }
    copy->index_ = index_;
    *clone = copy;
    return S_OK;
}

STDMETHODIMP DisplayAttributeEnumerator::Next(ULONG count, ITfDisplayAttributeInfo **info,
                                              ULONG *fetched) {
    if (info == nullptr) {
        return E_INVALIDARG;
    }
    ULONG produced = 0;
    if (count > 0 && index_ == 0 && info_ != nullptr) {
        info_->AddRef();
        info[0] = info_;
        produced = 1;
        index_ = 1;
    }
    if (fetched != nullptr) {
        *fetched = produced;
    }
    return produced == count ? S_OK : S_FALSE;
}

STDMETHODIMP DisplayAttributeEnumerator::Reset() {
    index_ = 0;
    return S_OK;
}

STDMETHODIMP DisplayAttributeEnumerator::Skip(ULONG count) {
    index_ = count > 0 ? 1 : index_;
    return S_OK;
}

} // namespace rustmagic

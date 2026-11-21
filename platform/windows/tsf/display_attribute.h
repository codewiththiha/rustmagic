/*
 * display_attribute.h — the one display attribute this service publishes.
 */

#ifndef RUSTMAGIC_DISPLAY_ATTRIBUTE_H
#define RUSTMAGIC_DISPLAY_ATTRIBUTE_H

#include <windows.h>
#include <msctf.h>

namespace rustmagic {

class DisplayAttributeInfo : public ITfDisplayAttributeInfo {
public:
    DisplayAttributeInfo();

    STDMETHODIMP QueryInterface(REFIID riid, void **ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    STDMETHODIMP GetGUID(GUID *guid) override;
    STDMETHODIMP GetDescription(BSTR *description) override;
    STDMETHODIMP GetAttributeInfo(TF_DISPLAYATTRIBUTE *attribute) override;
    STDMETHODIMP SetAttributeInfo(const TF_DISPLAYATTRIBUTE *attribute) override;
    STDMETHODIMP Reset() override;

private:
    ~DisplayAttributeInfo() = default;
    LONG refs_ = 1;
};

class DisplayAttributeEnumerator : public IEnumTfDisplayAttributeInfo {
public:
    DisplayAttributeEnumerator();

    STDMETHODIMP QueryInterface(REFIID riid, void **ppv) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    STDMETHODIMP Clone(IEnumTfDisplayAttributeInfo **clone) override;
    STDMETHODIMP Next(ULONG count, ITfDisplayAttributeInfo **info, ULONG *fetched) override;
    STDMETHODIMP Reset() override;
    STDMETHODIMP Skip(ULONG count) override;

private:
    ~DisplayAttributeEnumerator();
    LONG refs_ = 1;
    ULONG index_ = 0;
    ITfDisplayAttributeInfo *info_ = nullptr;
};

} // namespace rustmagic

#endif /* RUSTMAGIC_DISPLAY_ATTRIBUTE_H */

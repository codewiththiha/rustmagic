# RustMagic, built from the release tarball.
#
#   rpmbuild -ba packaging/rpm/rustmagic.spec \
#       --define "_sourcedir $PWD/dist" \
#       --define "_topdir $PWD/dist/rpmbuild"
#
# CI runs exactly that against the source archive of the tag, so the .rpm and
# the .deb cannot describe different programs.

Name:           rustmagic
Version:        %{version}
Release:        1%{?dist}
Summary:        Keyboard layout engine for Myanmar and other scripts, in Rust
License:        GPL-2.0-or-later
URL:            https://github.com/codewiththiha/rustmagic
Source0:        rustmagic-%{version}.tar.gz
BuildRequires:  cargo
BuildRequires:  rust
Requires:       ibus
# The window opens libGL and the X/Wayland stack at runtime, so the loader
# dependencies do not mention them and the spec has to.
Requires:       mesa-libGL
Requires:       libxkbcommon
Requires:       libX11
Requires:       libXcursor
Requires:       libXrandr
Requires:       libXi
Requires:       wayland-libs-client
Provides:       keymagic = %{version}
Conflicts:      keymagic < 3
ExclusiveArch:  x86_64

%description
RustMagic is a port of KeyMagic. It reads the same .km2 keyboards, compiles the
same .kms sources and matches the same rules, and it ships five Myanmar layouts
so it types out of the box.

It provides an IBus engine, so the layouts work in every application on a Linux
desktop, and a C ABI (libkm_ffi.so) for other input-method frameworks.

%prep
%autosetup -n rustmagic-%{version}

%build
cargo build --release --locked --package rustmagic --package rustmagic-cli \
    --package rustmagic-ibus --package km-ffi

%install
install -Dm755 target/release/rustmagic %{buildroot}%{_bindir}/rustmagic
install -Dm755 target/release/rustmagic-cli %{buildroot}%{_bindir}/rustmagic-cli
install -Dm755 target/release/rustmagic-ibus %{buildroot}%{_bindir}/rustmagic-ibus
install -Dm644 target/release/libkm_ffi.so %{buildroot}%{_libdir}/rustmagic/libkm_ffi.so
install -Dm644 crates/km-ffi/include/rustmagic.h %{buildroot}%{_includedir}/rustmagic.h
install -Dm644 assets/ibus/rustmagic.xml %{buildroot}%{_datadir}/ibus/component/rustmagic.xml
install -Dm644 packaging/linux/rustmagic.desktop %{buildroot}%{_datadir}/applications/rustmagic.desktop
install -Dm644 packaging/linux/rustmagic.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/rustmagic.svg
install -Dm644 packaging/linux/rustmagic-ibus.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/rustmagic-ibus.svg
install -d %{buildroot}%{_datadir}/rustmagic/keyboards
install -m644 keyboards/bundled/*.km2 %{buildroot}%{_datadir}/rustmagic/keyboards/
install -Dm644 LICENSE %{buildroot}%{_licensedir}/rustmagic/LICENSE
install -Dm644 assets/fonts/OFL.txt %{buildroot}%{_licensedir}/rustmagic/OFL-NotoSansMyanmar.txt
install -Dm644 README.md %{buildroot}%{_docdir}/rustmagic/README.md
install -Dm644 docs/LINUX.md %{buildroot}%{_docdir}/rustmagic/LINUX.md

%check
cargo test --release --locked --workspace

%post
# IBus caches the component files it finds; without this the engine exists but
# no keyboard chooser lists it.
if [ -x /usr/bin/ibus ]; then
    ibus write-cache >/dev/null 2>&1 || :
fi

%postun
if [ -x /usr/bin/ibus ]; then
    ibus write-cache >/dev/null 2>&1 || :
fi

%files
%{_bindir}/rustmagic
%{_bindir}/rustmagic-cli
%{_bindir}/rustmagic-ibus
%{_libdir}/rustmagic/libkm_ffi.so
%{_includedir}/rustmagic.h
%{_datadir}/ibus/component/rustmagic.xml
%{_datadir}/applications/rustmagic.desktop
%{_datadir}/icons/hicolor/scalable/apps/rustmagic.svg
%{_datadir}/icons/hicolor/scalable/apps/rustmagic-ibus.svg
%{_datadir}/rustmagic/keyboards/*.km2
# Named by path, not by bare file name: a bare `%license LICENSE` asks rpm for
# %{_licensedir}/%{name}-%{version}/LICENSE, which is not where %install put it.
%license %{_licensedir}/rustmagic/LICENSE
%license %{_licensedir}/rustmagic/OFL-NotoSansMyanmar.txt
%doc %{_docdir}/rustmagic/README.md
%doc %{_docdir}/rustmagic/LINUX.md

%changelog
* Sat Nov 21 2026 RustMagic contributors <noreply@rustmagic.invalid> - 0.2.0
- First release with installers for every platform.

Name:           mark-shot
Version:        0.1.22
Release:        1%{?dist}
Summary:        Qt 6 screenshot selection and annotation tool

License:        MIT
URL:            https://github.com/jswysnemc/mark-shot
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cmake >= 3.20
BuildRequires:  cmake-rpm-macros
BuildRequires:  gcc-c++
BuildRequires:  ninja-build
BuildRequires:  pkgconfig
BuildRequires:  cmake(Qt6Concurrent)
BuildRequires:  cmake(Qt6Core)
BuildRequires:  cmake(Qt6DBus)
BuildRequires:  cmake(Qt6Gui)
BuildRequires:  cmake(Qt6Widgets)
BuildRequires:  cmake(LayerShellQt)
BuildRequires:  pkgconfig(libpipewire-0.3)
BuildRequires:  pkgconfig(x11-xcb)
BuildRequires:  pkgconfig(xcb)

Requires:       python3
Requires:       qt6-qtwayland
Requires:       grim
Requires:       wl-clipboard
Recommends:     xclip
Recommends:     xdg-desktop-portal
Recommends:     pipewire
Suggests:       tesseract
Suggests:       tesseract-langpack-chi_sim
Suggests:       gnome-shell

%description
Mark Shot captures screenshots, annotates image regions, pins floating image
stickers, and provides OCR and translation helpers for pinned image text.

%prep
%autosetup

%build
%cmake -G Ninja \
    -DMARK_SHOT_WITH_LAYER_SHELL=ON \
    -DMARK_SHOT_REQUIRE_LAYER_SHELL=ON
%cmake_build

%install
%cmake_install
sed -i '1s|^#!/usr/bin/env python3$|#!/usr/bin/python3|' \
    %{buildroot}%{_bindir}/mark-shot-ocr \
    %{buildroot}%{_bindir}/mark-shot-code-scan \
    %{buildroot}%{_bindir}/mark-shot-translate \
    %{buildroot}%{_bindir}/mark-shot-upload \
    %{buildroot}%{_bindir}/mark-shot-window-detection-niri \
    %{buildroot}%{_bindir}/mark-shot-window-detection-hyprland \
    %{buildroot}%{_bindir}/mark-shot-window-detection-gnome \
    %{buildroot}%{_bindir}/mark-shot-window-detection-kde

%files
%license LICENSE
%doc README.md README.zh-CN.md
%{_bindir}/mark-shot
%{_bindir}/mark-shot-ocr
%{_bindir}/mark-shot-code-scan
%{_bindir}/mark-shot-translate
%{_bindir}/mark-shot-upload
%{_bindir}/mark-shot-window-detection-niri
%{_bindir}/mark-shot-window-detection-hyprland
%{_bindir}/mark-shot-window-detection-gnome
%{_bindir}/mark-shot-window-detection-kde
%{_libdir}/mark-shot/
%{_datadir}/applications/mark-shot.desktop
%{_datadir}/applications/mark-shot-edit.desktop
%{_datadir}/applications/net.local.mark-shot.desktop
%{_datadir}/icons/hicolor/scalable/apps/mark-shot.svg
%{_datadir}/icons/hicolor/scalable/apps/mark-shot-edit.svg
%{_datadir}/gnome-shell/extensions/mark-shot-scroll-helper@snemc.org/

%changelog
* Sun Jun 07 2026 jswysnemc <snemc@qq.com> - 0.1.22-1
- Update to version 0.1.22

* Sat Jun 06 2026 jswysnemc <snemc@qq.com> - 0.1.21-1
- Update to version 0.1.21

* Fri Jun 05 2026 jswysnemc <snemc@qq.com> - 0.1.20-1
- Update to version 0.1.20

* Fri Jun 05 2026 jswysnemc <snemc@qq.com> - 0.1.19-1
- Update to version 0.1.19

* Thu Jun 04 2026 jswysnemc <snemc@qq.com> - 0.1.18-1
- Update to version 0.1.18

* Thu Jun 04 2026 jswysnemc <snemc@qq.com> - 0.1.17-1
- Update to version 0.1.17

* Thu Jun 04 2026 jswysnemc <snemc@qq.com> - 0.1.16-1
- Initial Fedora RPM packaging

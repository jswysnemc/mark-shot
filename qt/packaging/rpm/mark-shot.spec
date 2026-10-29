Name:           mark-shot
Version:        0.1.61
Release:        1%{?dist}
Summary:        Qt 6 screenshot selection and annotation tool

License:        MIT
URL:            https://github.com/jswysnemc/mark-shot
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo >= 1.85
BuildRequires:  cmake >= 3.20
BuildRequires:  cmake-rpm-macros
BuildRequires:  gcc-c++
BuildRequires:  ninja-build
BuildRequires:  pkgconfig
BuildRequires:  rust >= 1.85
BuildRequires:  cmake(Qt6Concurrent)
BuildRequires:  cmake(Qt6Core)
BuildRequires:  cmake(Qt6DBus)
BuildRequires:  cmake(Qt6Gui)
BuildRequires:  cmake(Qt6Network)
BuildRequires:  cmake(Qt6Svg)
BuildRequires:  cmake(Qt6Widgets)
BuildRequires:  cmake(LayerShellQt)
# OCR 与扫码插件必须在打包时与本机 Qt 一起构建；插件市场的 Linux 资产基于 Arch 的 Qt，
# 版本高于 Fedora 时 QPluginLoader 会拒绝加载
BuildRequires:  pkgconfig(libonnxruntime)
BuildRequires:  pkgconfig(zxing)
BuildRequires:  pkgconfig(libavcodec)
BuildRequires:  pkgconfig(libavfilter)
BuildRequires:  pkgconfig(libavformat)
BuildRequires:  pkgconfig(libavutil)
BuildRequires:  pkgconfig(libpipewire-0.3)
BuildRequires:  pkgconfig(libswresample)
BuildRequires:  pkgconfig(libswscale)
BuildRequires:  pkgconfig(x11-xcb)
BuildRequires:  pkgconfig(xcb)

Requires:       python3
Requires:       qt6-qtwayland
# qt6-qtsvg 提供 qsvg 图像插件与 qsvgicon 图标引擎，Qt 运行时按需加载，
# 自动依赖生成器扫不出来；缺失时设置界面的 SVG 图标无法渲染
Requires:       qt6-qtsvg
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
    -DCMAKE_INSTALL_LIBEXECDIR=%{_libexecdir} \
    -DMARK_SHOT_REQUIRE_FFMPEG=ON \
    -DMARK_SHOT_WITH_LAYER_SHELL=ON \
    -DMARK_SHOT_REQUIRE_LAYER_SHELL=ON
%cmake_build

%install
%cmake_install
sed -i '1s|^#!/usr/bin/env python3$|#!/usr/bin/python3|' \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-ocr \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-code-scan \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-translate \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-upload \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-window-detection-niri \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-window-detection-hyprland \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-window-detection-gnome \
    %{buildroot}%{_libexecdir}/mark-shot/mark-shot-window-detection-kde

%files
%license LICENSE
%doc %{_docdir}/mark-shot/
%{_bindir}/mark-shot
%{_libexecdir}/mark-shot/
%{_libdir}/mark-shot/
%{_datadir}/applications/mark-shot.desktop
%{_datadir}/applications/mark-shot-edit.desktop
%{_datadir}/applications/net.local.mark-shot.desktop
%{_datadir}/icons/hicolor/scalable/apps/mark-shot.svg
%{_datadir}/icons/hicolor/scalable/apps/mark-shot-edit.svg
%{_datadir}/icons/hicolor/*x*/apps/mark-shot.png
%{_datadir}/icons/hicolor/*x*/apps/mark-shot-edit.png
%{_datadir}/gnome-shell/extensions/mark-shot-scroll-helper@snemc.org/

%changelog
* Sun Oct 25 2026 jswysnemc <snemc@qq.com> - 0.1.61-1
- Fix KWin mixed-scale captures, add region detection switch, route startup tools to cursor screen, and bundle detection scripts

* Mon Oct 19 2026 jswysnemc <snemc@qq.com> - 0.1.60-1
- Add window borders, dynamic marker toolbar hover icons, and native plugin recommendations

* Fri Oct 16 2026 jswysnemc <snemc@qq.com> - 0.1.59-1
- Move core image and selection work into Rust and release OCR sessions after recognition

* Tue Oct 13 2026 jswysnemc <snemc@qq.com> - 0.1.58-1
- Add UI-region hover detection on frozen screenshots and wheel region cycling

* Mon Oct 12 2026 jswysnemc <snemc@qq.com> - 0.1.57-1
- Add arrow shaft styles and keep scrolling capture stitching with fixed sidebars

* Fri Oct 02 2026 jswysnemc <snemc@qq.com> - 0.1.56-1
- Remember screen-sharing authorization and fix Wayland capture fallbacks

* Wed Sep 30 2026 jswysnemc <snemc@qq.com> - 0.1.55-1
- Update to version 0.1.55

* Fri Sep 25 2026 jswysnemc <snemc@qq.com> - 0.1.54-1
- Update to version 0.1.54

* Mon Sep 21 2026 jswysnemc <snemc@qq.com> - 0.1.53-1
- Update to version 0.1.53

* Sun Sep 13 2026 jswysnemc <snemc@qq.com> - 0.1.52-1
- Update to version 0.1.52

* Sun Sep 06 2026 jswysnemc <snemc@qq.com> - 0.1.51-1
- Update to version 0.1.51

* Sun Aug 30 2026 jswysnemc <snemc@qq.com> - 0.1.50-1
- Update to version 0.1.50

* Sun Aug 23 2026 jswysnemc <snemc@qq.com> - 0.1.49-1
- Update to version 0.1.49

* Sun Aug 16 2026 jswysnemc <snemc@qq.com> - 0.1.48-1
- Update to version 0.1.48

* Mon Aug 10 2026 jswysnemc <snemc@qq.com> - 0.1.47-1
- Update to version 0.1.47

* Mon Aug 10 2026 jswysnemc <snemc@qq.com> - 0.1.46-1
- Update to version 0.1.46

* Thu Aug 06 2026 jswysnemc <snemc@qq.com> - 0.1.45-1
- Update to version 0.1.45

* Sun Jul 26 2026 jswysnemc <snemc@qq.com> - 0.1.44-1
- Update to version 0.1.44

* Sun Jul 26 2026 jswysnemc <snemc@qq.com> - 0.1.43-1
- Update to version 0.1.43

* Sun Jul 26 2026 jswysnemc <snemc@qq.com> - 0.1.42-1
- Update to version 0.1.42

* Thu Jul 16 2026 jswysnemc <snemc@qq.com> - 0.1.41-1
- Update to version 0.1.41

* Thu Jul 16 2026 jswysnemc <snemc@qq.com> - 0.1.40-1
- Update to version 0.1.40

* Fri Jul 10 2026 jswysnemc <snemc@qq.com> - 0.1.39-1
- Update to version 0.1.39

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

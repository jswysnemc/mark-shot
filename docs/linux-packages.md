# Linux Package Selection

Download release artifacts from the [GitHub Releases page](https://github.com/jswysnemc/mark-shot/releases). Select the package built for the target distribution because FFmpeg SONAME dependencies are not interchangeable between releases.

## Debian and Ubuntu

| Distribution | Package | Runtime baseline |
| --- | --- | --- |
| Debian 12 / Deepin | `mark-shot_<version>_amd64.deb` | FFmpeg 5 and older Qt packages |
| Debian 13 / LMDE 7 | `mark-shot_<version>_amd64.debian13.deb` | FFmpeg 7 and Debian 13 zxing-cpp |
| Ubuntu 24.04 | `mark-shot_<version>_amd64.ubuntu24.04.deb` | FFmpeg 6 and Qt 6.4 t64 packages |
| Ubuntu 26.04 | `mark-shot_<version>_amd64.ubuntu26.04.deb` | Current FFmpeg and Qt t64 packages |

Install the downloaded package with:

```bash
sudo apt install ./<downloaded-mark-shot-package>.deb
```

The Debian 12 package omits LayerShellQt so it remains installable on Deepin and older Debian-derived systems. Wayland overlays can fall back to regular windows on compositors such as niri.

The Ubuntu 26.04 package enables the layer-shell plugin when the distribution provides compatible development packages. Generated dependencies include the main executable and installed shared-library plugins.

## AppImage

The AppImage payload is built on Debian 12, rejects glibc symbol requirements newer than `GLIBC_2.36`, and is started in an Ubuntu 24.04 container before release. This baseline supports Ubuntu 24.04 and Debian 13 without inheriting the build host's newer glibc requirement.

Debian 12 does not provide the Qt 6 LayerShellQt development package, so the compatibility AppImage omits that optional plugin and uses the regular-window fallback for overlays. Distribution-native packages can enable the plugin when a compatible package is available.

## Fedora

Install the RPM package with:

```bash
sudo dnf install ./mark-shot-<version>-1.x86_64.rpm
```

Provider plugins are included only when their build dependencies are present. Do not mix standalone plugin shared libraries built for different distributions.

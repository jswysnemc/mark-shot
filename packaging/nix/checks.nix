/**
  【Nix验证】【安装检查】按软件包与版本返回安装、CLI、动态链接及媒体 ABI 检查
*/
{
  pkgs,
  version,
  mark-shot,
}:
{
  package = mark-shot;
  installed =
    pkgs.runCommand "mark-shot-installed-check"
      {
        nativeBuildInputs = [
          pkgs.python3
          pkgs.binutils
          pkgs.desktop-file-utils
        ];
      }
      ''
            mkdir -p "$out" "$TMPDIR/config" "$TMPDIR/data" "$TMPDIR/state"
            export XDG_CONFIG_HOME="$TMPDIR/config" XDG_DATA_HOME="$TMPDIR/data" XDG_STATE_HOME="$TMPDIR/state"
            test "$(${mark-shot}/bin/mark-shot --version)" = "mark-shot ${version}"
            test "$(env -u LD_LIBRARY_PATH ${mark-shot}/lib/mark-shot/mark-shot --version)" = "mark-shot ${version}"
            ${mark-shot}/bin/mark-shot --help > "$out/help.txt"
            grep -aFq '${pkgs.lib.getExe' pkgs.fontconfig "fc-match"}' ${mark-shot}/lib/mark-shot/mark-shot
            desktop-file-validate ${mark-shot}/share/applications/io.github.jswysnemc.MarkShot.desktop
            test -f ${mark-shot}/share/icons/hicolor/scalable/apps/io.github.jswysnemc.MarkShot.svg
            test -s ${mark-shot}/share/licenses/mark-shot/DEPENDENCIES.txt
            readelf -d ${mark-shot}/lib/mark-shot/mark-shot > "$out/linkage.txt"
            if grep -Ei 'libQt|libgtk|webkit|webview|libavcodec|onnxruntime|webgpu_dawn' "$out/linkage.txt"; then
              exit 1
            fi
            (cd ${mark-shot}/lib/mark-shot/media; sha256sum --check SHA256SUMS)
            LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [ pkgs.libpulseaudio ]} \
              python - ${mark-shot}/lib/mark-shot/media/libmarkshot-media.so <<'PY'
        import ctypes
        import sys
        library = ctypes.CDLL(sys.argv[1])
        library.ms_media_abi.restype = ctypes.c_uint32
        assert library.ms_media_abi() == 2
        for name in ("ms_encoder_open", "ms_encoder_video", "ms_encoder_finish", "ms_export",
                     "ms_pulse_open", "ms_surface_open", "ms_encoder_dmabuf"):
            getattr(library, name)
        ctypes.CDLL("libpulse.so.0")
        PY
      '';
}

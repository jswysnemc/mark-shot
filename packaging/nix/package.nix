/**
  【Nix打包】【原生程序】按源码、版本、工具链和媒体库返回包含桌面入口及许可的 Rust 软件包
*/
{
  lib,
  stdenv,
  rustPlatform,
  python3,
  fontconfig,
  coreutils,
  dbus,
  patchelf,
  source,
  version,
  sourceRevision,
  dependencies,
  mediaLibrary,
}:
rustPlatform.buildRustPackage {
  pname = "mark-shot";
  inherit version;
  src = source;
  cargoLock.lockFile = ../../Cargo.lock;
  nativeBuildInputs = dependencies.nativeBuildInputs ++ [ patchelf ];
  nativeCheckInputs = dependencies.nativeCheckInputs;
  buildInputs = dependencies.buildInputs;
  cargoBuildFlags = [
    "--bin"
    "mark-shot"
  ];
  cargoTestFlags = [ "--workspace" ];
  checkType = "debug";
  doCheck = true;
  CARGO_PROFILE_DEV_DEBUG = "0";
  CARGO_PROFILE_TEST_DEBUG = "0";
  LANG = "C.UTF-8";
  LC_ALL = "C.UTF-8";

  postPatch = ''
    # 1. 【Nix验证】【测试环境】固定测试程序路径与私有总线配置，沙箱不依赖主机的 /bin、/usr/bin 和 /etc
    substituteInPlace src/translation/process/tests.rs \
      src/translation/task/tests/image_tests.rs src/translation/task/tests/process_tests.rs \
      src/features/ocr_result/translation/controller/lifecycle_tests.rs \
      tests/upload/process.rs tests/external_actions.rs \
      --replace-fail /usr/bin/python3 ${python3}/bin/python3
    substituteInPlace src/utils/helper_process/linux.rs \
      --replace-fail /bin/cat ${lib.getExe' coreutils "cat"}
    substituteInPlace src/features/ocr_result/translation/controller/lifecycle_tests.rs \
      --replace-fail /bin/sleep ${lib.getExe' coreutils "sleep"}
    substituteInPlace src/backend/window_detection/kde/tests.rs \
      src/backend/wayland/capture/kde/connection_tests.rs \
      --replace-fail '"--session"' '"--config-file=${dbus}/share/dbus-1/session.conf"'
    # 2. 【Nix打包】【字体匹配】固定外部匹配器路径，后台直接启动真实程序时同样能读取系统字体
    substituteInPlace src/fonts/text_faces.rs \
      --replace-fail 'Command::new("fc-match")' 'Command::new("${lib.getExe' fontconfig "fc-match"}")'
  '';

  preCheck = ''
    # 3. 【Nix验证】【环境隔离】测试只写临时配置目录，参考字体使用固定归档与 Nix 字体配置
    export XDG_CONFIG_HOME="$TMPDIR/mark-shot-test/config"
    export XDG_DATA_HOME="$TMPDIR/mark-shot-test/data"
    export XDG_STATE_HOME="$TMPDIR/mark-shot-test/state"
    mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME"
    python scripts/ci-reference-fonts.py --archive ${dependencies.referenceFontArchive} \
      --base-config ${dependencies.testFontConfig} --root "$TMPDIR/mark-shot-test/fonts"
    export FONTCONFIG_FILE="$TMPDIR/mark-shot-test/fonts/fonts.conf"
    export MARK_SHOT_MEDIA_LIBRARY="${mediaLibrary}/lib/mark-shot/media/libmarkshot-media.so"
    export MARK_SHOT_MEDIA_EVIDENCE="$TMPDIR/mark-shot-test/media"
    mkdir -p "$MARK_SHOT_MEDIA_EVIDENCE"
  '';
  postCheck = ''
    # 4. 【Nix验证】【媒体编码】验证真实进程内软件编码与导出，不依赖交互桌面或外部编码程序
    cargo test -p shot-media --release --locked --offline \
      --target ${stdenv.hostPlatform.rust.rustcTarget} -- --ignored
  '';
  postInstall = ''
    # 5. 【Nix打包】【程序布局】保持真实程序旁的媒体目录，包装入口提供动态音频库和本地 OCR 的 C++ 运行库
    mkdir -p "$out/lib/mark-shot"
    mv "$out/bin/mark-shot" "$out/lib/mark-shot/mark-shot"
    ln -s ${mediaLibrary}/lib/mark-shot/media "$out/lib/mark-shot/media"
    printf '%s\n' ${lib.escapeShellArg sourceRevision} > "$out/lib/mark-shot/SOURCE-COMMIT"
    install -Dm644 packaging/desktop/io.github.jswysnemc.MarkShot.desktop \
      "$out/share/applications/io.github.jswysnemc.MarkShot.desktop"
    install -Dm644 packaging/desktop/io.github.jswysnemc.MarkShot.metainfo.xml \
      "$out/share/metainfo/io.github.jswysnemc.MarkShot.metainfo.xml"
    install -Dm644 assets/hicolor/scalable/apps/mark-shot.svg \
      "$out/share/icons/hicolor/scalable/apps/io.github.jswysnemc.MarkShot.svg"
    # 6. 【Nix打包】【依赖许可】从离线 Cargo 来源生成版权通知，保留字体、图标和项目许可
    mkdir -p "$out/share/licenses/mark-shot" "$out/share/doc/mark-shot"
    cargo-about generate --frozen --workspace --target ${stdenv.hostPlatform.rust.rustcTarget} \
      --fail --config release/about.toml --output-file "$out/share/licenses/mark-shot/DEPENDENCIES.txt" \
      release/license-notices.hbs
    cp LICENSE-MIT LICENSE-APACHE assets/fonts/LICENSE-OFL.txt assets/fonts/LICENSE-NUMERALS.txt \
      "$out/share/licenses/mark-shot/"
    cp assets/icons/mark-shot/LICENSE "$out/share/licenses/mark-shot/LICENSE-ICONS.txt"
    cp Cargo.lock "$out/share/doc/mark-shot/Cargo.lock"
  '';
  postFixup = ''
    # 7. 【Nix打包】【后台依赖】真实程序保留可传递的库路径，登录启动和辅助进程同样支持动态 OCR 与音频库
    binary="$out/lib/mark-shot/mark-shot"
    patchelf --force-rpath --set-rpath "$(patchelf --print-rpath "$binary"):${dependencies.runtimeLibraryPath}" "$binary"
    makeWrapper "$out/lib/mark-shot/mark-shot" "$out/bin/mark-shot" \
      --prefix LD_LIBRARY_PATH : ${lib.escapeShellArg dependencies.runtimeLibraryPath}
  '';
  passthru = { inherit mediaLibrary sourceRevision; };
  meta = {
    description = "Native screenshot, annotation and recording tool";
    homepage = "https://github.com/jswysnemc/mark-shot";
    license = with lib.licenses; [
      mit
      asl20
    ];
    mainProgram = "mark-shot";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
  };
}

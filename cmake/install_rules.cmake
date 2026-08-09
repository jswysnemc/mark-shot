install(TARGETS mark-shot RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

install(FILES
    README.md
    README.zh-CN.md
    CHANGELOG.md
    DESTINATION ${CMAKE_INSTALL_DOCDIR}
)
install(DIRECTORY docs/
    DESTINATION ${CMAKE_INSTALL_DOCDIR}/docs
)

if(TARGET mark-shot-layer-shell)
    install(TARGETS mark-shot-layer-shell
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    )
endif()

if(MARK_SHOT_LINUX)
    set(MARK_SHOT_DESKTOP_EXEC "${CMAKE_INSTALL_FULL_BINDIR}/mark-shot")
    configure_file(data/mark-shot.desktop.in "${CMAKE_CURRENT_BINARY_DIR}/mark-shot.desktop" @ONLY)
    configure_file(data/mark-shot-edit.desktop.in "${CMAKE_CURRENT_BINARY_DIR}/mark-shot-edit.desktop" @ONLY)
    configure_file(data/net.local.mark-shot.desktop.in "${CMAKE_CURRENT_BINARY_DIR}/net.local.mark-shot.desktop" @ONLY)

    install(PROGRAMS
        scripts/mark-shot-ocr
        scripts/mark-shot-code-scan
        scripts/mark-shot-translate
        scripts/mark-shot-upload
        scripts/mark-shot-window-detection-niri
        scripts/mark-shot-window-detection-hyprland
        scripts/mark-shot-window-detection-gnome
        scripts/mark-shot-window-detection-kde
        DESTINATION ${CMAKE_INSTALL_BINDIR}
    )
    install(DIRECTORY scripts/lib/mark_shot_window_detection
        DESTINATION ${CMAKE_INSTALL_DATADIR}/mark-shot/python
        FILES_MATCHING
            PATTERN "*.py"
            PATTERN "__pycache__" EXCLUDE
    )
    install(FILES
        "${CMAKE_CURRENT_BINARY_DIR}/mark-shot.desktop"
        "${CMAKE_CURRENT_BINARY_DIR}/mark-shot-edit.desktop"
        "${CMAKE_CURRENT_BINARY_DIR}/net.local.mark-shot.desktop"
        DESTINATION ${CMAKE_INSTALL_DATADIR}/applications
    )
    install(FILES
        data/icons/hicolor/scalable/apps/mark-shot.svg
        data/icons/hicolor/scalable/apps/mark-shot-edit.svg
        DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps
    )
    # 位图图标由 qt6-base 自带的图像插件解码，不依赖独立的 Qt SVG 包
    # （qt6-svg / libqt6svg6）。只安装 SVG 时，缺少该包的环境下图标主题查找
    # 会得到无法渲染的空图标，系统托盘因此显示为空白。
    foreach(icon_size 16 22 24 32 48 64 128 256)
        install(FILES
            data/icons/hicolor/${icon_size}x${icon_size}/apps/mark-shot.png
            data/icons/hicolor/${icon_size}x${icon_size}/apps/mark-shot-edit.png
            DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/${icon_size}x${icon_size}/apps
        )
    endforeach()
    install(DIRECTORY
        packaging/gnome-extension/mark-shot-scroll-helper@snemc.org/
        DESTINATION ${CMAKE_INSTALL_DATADIR}/gnome-shell/extensions/mark-shot-scroll-helper@snemc.org
        FILES_MATCHING
            PATTERN "metadata.json"
            PATTERN "*.js"
    )
endif()

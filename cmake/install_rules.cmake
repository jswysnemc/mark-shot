install(TARGETS mark-shot RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

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
    install(DIRECTORY
        packaging/gnome-extension/mark-shot-scroll-helper@snemc.org/
        DESTINATION ${CMAKE_INSTALL_DATADIR}/gnome-shell/extensions/mark-shot-scroll-helper@snemc.org
        FILES_MATCHING
            PATTERN "metadata.json"
            PATTERN "*.js"
    )
endif()

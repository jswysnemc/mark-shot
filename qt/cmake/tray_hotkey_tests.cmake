option(MARK_SHOT_X11_HOTKEY_INTEGRATION_TESTS "Run tray global hotkey reload checks in an isolated Xvfb session" OFF)

# 1. 【托盘测试】【全局快捷键】在独立 Xvfb 中真实抓键，验证保存设置后无需重启即可切换快捷键
if(MARK_SHOT_X11_HOTKEY_INTEGRATION_TESTS AND UNIX AND NOT APPLE)
    find_program(MARK_SHOT_XVFB_RUN xvfb-run)
    find_program(MARK_SHOT_XDOTOOL xdotool)
    if(NOT MARK_SHOT_XVFB_RUN OR NOT MARK_SHOT_XDOTOOL)
        message(STATUS "X11 hotkey integration tests skipped: xvfb-run or xdotool not found")
        return()
    endif()

    qt_add_executable(mark-shot-tray-hotkey-reload-test
        tests/tray_hotkey_reload_test.cpp
        "$<FILTER:$<TARGET_OBJECTS:mark-shot>,EXCLUDE,/main\\.cpp\\.(o|obj)$>"
    )
    target_include_directories(mark-shot-tray-hotkey-reload-test PRIVATE "$<TARGET_PROPERTY:mark-shot,INCLUDE_DIRECTORIES>")
    target_compile_definitions(mark-shot-tray-hotkey-reload-test PRIVATE "$<TARGET_PROPERTY:mark-shot,COMPILE_DEFINITIONS>")
    target_link_libraries(mark-shot-tray-hotkey-reload-test PRIVATE "$<TARGET_PROPERTY:mark-shot,LINK_LIBRARIES>" Qt6::Test)
    get_target_property(tray_hotkey_cxx_standard mark-shot CXX_STANDARD)
    set_target_properties(mark-shot-tray-hotkey-reload-test PROPERTIES CXX_STANDARD ${tray_hotkey_cxx_standard})
    add_dependencies(mark-shot-tray-hotkey-reload-test mark-shot)

    # 2. 【托盘测试】【隔离会话】断开 Wayland 与会话总线，只留下 X11 抓键后端，避免触碰 portal 或 GNOME 快捷键
    add_test(NAME tray-hotkey-reload-x11
        COMMAND ${MARK_SHOT_XVFB_RUN} -a -s "-screen 0 1280x800x24"
            $<TARGET_FILE:mark-shot-tray-hotkey-reload-test>)
    set_tests_properties(tray-hotkey-reload-x11 PROPERTIES TIMEOUT 60 LABELS "x11;integration"
        ENVIRONMENT "QT_QPA_PLATFORM=xcb;WAYLAND_DISPLAY=;XDG_SESSION_TYPE=x11;XDG_CURRENT_DESKTOP=;DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/mark-shot-test-bus;MARK_SHOT_TEST_XDOTOOL=${MARK_SHOT_XDOTOOL}")
endif()

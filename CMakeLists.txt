cmake_minimum_required(VERSION 3.20)

project(mark-shot VERSION 0.1.24 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_INSTALL_RPATH_USE_LINK_PATH ON)

include(GNUInstallDirs)

option(MARK_SHOT_WITH_LAYER_SHELL "Build the optional LayerShellQt plugin when available" ON)
option(MARK_SHOT_REQUIRE_LAYER_SHELL "Fail configuration when the optional LayerShellQt plugin cannot be built" OFF)
option(MARK_SHOT_WITH_LIBPORTAL "Enable libportal screencast negotiation when available" OFF)
option(MARK_SHOT_REQUIRE_LIBPORTAL "Fail configuration when libportal support is requested but unavailable" OFF)

if(WIN32)
    enable_language(RC)
endif()

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(MARK_SHOT_LINUX TRUE)
else()
    set(MARK_SHOT_LINUX FALSE)
endif()

set(MARK_SHOT_QT_COMPONENTS Core Gui Widgets Concurrent Network)
if(MARK_SHOT_LINUX)
    list(APPEND MARK_SHOT_QT_COMPONENTS DBus)
endif()
find_package(Qt6 REQUIRED COMPONENTS ${MARK_SHOT_QT_COMPONENTS})

if(MARK_SHOT_WITH_LAYER_SHELL AND MARK_SHOT_LINUX)
    find_package(LayerShellQt QUIET)
    if(MARK_SHOT_REQUIRE_LAYER_SHELL AND NOT LayerShellQt_FOUND)
        message(FATAL_ERROR "The optional LayerShellQt plugin was required but LayerShellQt was not found")
    endif()
elseif(MARK_SHOT_REQUIRE_LAYER_SHELL)
    message(FATAL_ERROR "The optional LayerShellQt plugin is only supported on Linux")
endif()
if(MARK_SHOT_LINUX)
    find_package(X11 COMPONENTS xcb QUIET)
    find_package(PkgConfig QUIET)
    if(PkgConfig_FOUND)
        pkg_check_modules(PipeWire QUIET IMPORTED_TARGET libpipewire-0.3)
        if(MARK_SHOT_WITH_LIBPORTAL)
            pkg_check_modules(LibPortal QUIET IMPORTED_TARGET libportal)
            if(MARK_SHOT_REQUIRE_LIBPORTAL AND NOT LibPortal_FOUND)
                message(FATAL_ERROR "libportal support was required but libportal was not found")
            endif()
        endif()
    endif()
elseif(MARK_SHOT_REQUIRE_LIBPORTAL)
    message(FATAL_ERROR "libportal support is only supported on Linux")
endif()

if(COMMAND qt_standard_project_setup)
    qt_standard_project_setup()
else()
    set(CMAKE_AUTOMOC ON)
    set(CMAKE_AUTOUIC ON)
    set(CMAKE_AUTORCC ON)
endif()

qt_add_executable(mark-shot
    resources.qrc
    src/main.cpp
    src/clipboard_image.cpp
    src/clipboard_image.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/debug_log.cpp
    src/debug_log.h
    src/config_value.cpp
    src/config_value.h
    src/capture_geometry.cpp
    src/capture_geometry.h
    src/layer_shell_plugin_interface.h
    src/layer_shell_runtime.cpp
    src/layer_shell_runtime.h
    src/window_detection.cpp
    src/window_detection.h
    src/windows_integration.cpp
    src/windows_integration.h
    src/windows_tray_controller.cpp
    src/windows_tray_controller.h
    src/screen_capture.cpp
    src/screen_capture.h
    src/screen_capture_api.cpp
    src/screen_capture_core_helpers.cpp
    src/screen_capture_dbus_portal.cpp
    src/screen_capture_dbus_wayland.cpp
    src/screen_capture_internal.h
    src/screen_capture_pipewire.cpp
    src/screen_capture_pipewire_libportal.cpp
    src/screen_capture_pipewire_libportal.h
    src/screen_capture_platform_api.cpp
    src/screen_capture_windows_wgc.cpp
    src/screen_capture_x11.cpp
    src/shell_command.cpp
    src/shell_command.h
    src/ocr_result.cpp
    src/ocr_result.h
    src/shortcut_config.cpp
    src/shortcut_config.h
    src/shot_window.cpp
    src/shot_window.h
    src/shot_window_config.cpp
    src/shot_window_internal.cpp
    src/shot_window_internal.h
    src/shot_window_module.h
    src/shot_window_ocr_result_window.cpp
    src/shot_window_actions.cpp
    src/shot_window_annotation_editing.cpp
    src/shot_window_annotation_painting.cpp
    src/shot_window_canvas.cpp
    src/shot_window_hit_testing.cpp
    src/shot_window_input.cpp
    src/shot_window_layout.cpp
    src/shot_window_modes.cpp
    src/shot_window_property_panels.cpp
    src/shot_window_setup.cpp
    src/shot_window_pinned_window.cpp
    src/pinned_window_top.cpp
    src/pinned_window_top.h
    src/annotation_launch.cpp
    src/annotation_launch.h
    src/scroll/stitcher.cpp
    src/scroll/stitcher.h
    src/scroll/stitcher_internal.cpp
    src/scroll/stitcher_internal.h
    src/scroll/stitcher_algorithm.cpp
    src/scroll/scroll_session_window.cpp
    src/scroll/scroll_session_window_capture.cpp
    src/scroll/scroll_session_window_events.cpp
    src/scroll/scroll_session_window.h
    src/scroll/scroll_session_window_internal.h
    src/scroll/scroll_session_window_lifecycle.cpp
    src/scroll/scroll_session_window_preview.cpp
    src/ui/color_picker.cpp
    src/ui/color_picker.h
    src/ui/icons.cpp
    src/ui/icons.h
    src/ui/i18n.cpp
    src/ui/i18n.h
    src/ui/theme.cpp
    src/ui/theme.h
)

target_include_directories(mark-shot PRIVATE src)
target_compile_definitions(mark-shot PRIVATE MARK_SHOT_VERSION="${PROJECT_VERSION}")
if(WIN32)
    set_property(TARGET mark-shot PROPERTY CXX_STANDARD 20)
    set(MARK_SHOT_WINDOWS_ICON "${CMAKE_CURRENT_SOURCE_DIR}/packaging/windows/mark-shot.ico")
    configure_file(packaging/windows/mark-shot.rc.in "${CMAKE_CURRENT_BINARY_DIR}/mark-shot.rc" @ONLY)
    target_sources(mark-shot PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/mark-shot.rc")
    set_target_properties(mark-shot PROPERTIES WIN32_EXECUTABLE TRUE)
    target_link_libraries(mark-shot PRIVATE d3d11 dwmapi dxgi user32 windowsapp)
endif()

target_link_libraries(mark-shot
    PRIVATE
        Qt6::Core
        Qt6::Concurrent
        Qt6::Gui
        Qt6::Network
        Qt6::Widgets
)

if(MARK_SHOT_LINUX)
    target_link_libraries(mark-shot PRIVATE Qt6::DBus)
    target_compile_definitions(mark-shot PRIVATE MARK_SHOT_WITH_DBUS)
endif()

if(MARK_SHOT_LINUX AND X11_xcb_FOUND)
    target_link_libraries(mark-shot PRIVATE X11::xcb)
    target_compile_definitions(mark-shot PRIVATE HAVE_XCB)
endif()

if(MARK_SHOT_WITH_LAYER_SHELL AND MARK_SHOT_LINUX AND LayerShellQt_FOUND)
    qt_add_plugin(mark-shot-layer-shell MODULE)
    target_sources(mark-shot-layer-shell
        PRIVATE
        src/debug_log.cpp
        src/debug_log.h
        src/layer_shell_plugin.cpp
        src/layer_shell_plugin_interface.h
    )
    target_include_directories(mark-shot-layer-shell PRIVATE src)
    target_link_libraries(mark-shot-layer-shell
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Widgets
            LayerShellQt::Interface
    )
    set_target_properties(mark-shot-layer-shell PROPERTIES
        LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}"
    )
    message(STATUS "mark-shot: LayerShellQt found, optional layer-shell plugin enabled")
elseif(MARK_SHOT_WITH_LAYER_SHELL AND MARK_SHOT_LINUX)
    message(STATUS "mark-shot: LayerShellQt not found, building without optional layer-shell plugin")
elseif(MARK_SHOT_WITH_LAYER_SHELL)
    message(STATUS "mark-shot: LayerShellQt is Linux-only, building without optional layer-shell plugin")
else()
    message(STATUS "mark-shot: optional layer-shell plugin disabled")
endif()

if(MARK_SHOT_LINUX AND PipeWire_FOUND)
    target_link_libraries(mark-shot PRIVATE PkgConfig::PipeWire)
    target_compile_definitions(mark-shot PRIVATE HAVE_PIPEWIRE)
    message(STATUS "mark-shot: PipeWire ${PipeWire_VERSION} found, portal screencast capture enabled")
    if(MARK_SHOT_WITH_LIBPORTAL AND LibPortal_FOUND)
        target_link_libraries(mark-shot PRIVATE PkgConfig::LibPortal)
        target_compile_definitions(mark-shot PRIVATE HAVE_LIBPORTAL)
        message(STATUS "mark-shot: libportal ${LibPortal_VERSION} found, libportal screencast negotiation enabled")
    elseif(MARK_SHOT_WITH_LIBPORTAL)
        message(STATUS "mark-shot: libportal not found, using direct D-Bus portal screencast negotiation")
    else()
        message(STATUS "mark-shot: libportal support disabled, using direct D-Bus portal screencast negotiation")
    endif()
else()
    if(MARK_SHOT_LINUX)
        message(STATUS "mark-shot: PipeWire not found, Wayland scrolling capture falls back to screenshot portal/grim")
    else()
        message(STATUS "mark-shot: Linux screenshot backends disabled on ${CMAKE_SYSTEM_NAME}; using Qt screen capture")
    endif()
endif()

include(CTest)
if(BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)

    qt_add_executable(mark-shot-capture-geometry-test
        tests/capture_geometry_test.cpp
        src/capture_geometry.cpp
        src/capture_geometry.h
    )
    target_include_directories(mark-shot-capture-geometry-test PRIVATE src)
    target_link_libraries(mark-shot-capture-geometry-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
    )
    add_test(NAME capture-geometry COMMAND mark-shot-capture-geometry-test)

    qt_add_executable(mark-shot-config-value-test
        tests/config_value_test.cpp
        src/config_value.cpp
        src/config_value.h
    )
    target_include_directories(mark-shot-config-value-test PRIVATE src)
    target_link_libraries(mark-shot-config-value-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
    )
    add_test(NAME config-value COMMAND mark-shot-config-value-test)

    qt_add_executable(mark-shot-ocr-result-test
        tests/ocr_result_test.cpp
        src/ocr_result.cpp
        src/ocr_result.h
    )
    target_include_directories(mark-shot-ocr-result-test PRIVATE src)
    target_link_libraries(mark-shot-ocr-result-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
    )
    add_test(NAME ocr-result COMMAND mark-shot-ocr-result-test)

    qt_add_executable(mark-shot-stitcher-test
        tests/stitcher_test.cpp
        src/debug_log.cpp
        src/debug_log.h
        src/scroll/stitcher.cpp
        src/scroll/stitcher.h
        src/scroll/stitcher_internal.cpp
        src/scroll/stitcher_internal.h
        src/scroll/stitcher_algorithm.cpp
    )
    target_include_directories(mark-shot-stitcher-test PRIVATE src)
    target_link_libraries(mark-shot-stitcher-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
    )
    add_test(NAME stitcher COMMAND mark-shot-stitcher-test)
endif()

install(TARGETS mark-shot RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
if(TARGET mark-shot-layer-shell)
    install(TARGETS mark-shot-layer-shell
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    )
endif()
if(MARK_SHOT_LINUX)
    install(PROGRAMS
        scripts/mark-shot-ocr
        scripts/mark-shot-translate
        scripts/mark-shot-window-detection-niri
        scripts/mark-shot-window-detection-hyprland
        scripts/mark-shot-window-detection-gnome
        DESTINATION ${CMAKE_INSTALL_BINDIR}
    )
    install(FILES
        data/mark-shot.desktop
        data/mark-shot-edit.desktop
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
            PATTERN "extension.js"
    )
endif()

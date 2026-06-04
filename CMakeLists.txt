cmake_minimum_required(VERSION 3.20)

project(mark-shot VERSION 0.1.19 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_INSTALL_RPATH_USE_LINK_PATH ON)

include(GNUInstallDirs)

option(MARK_SHOT_WITH_LAYER_SHELL "Build the optional LayerShellQt plugin when available" ON)
option(MARK_SHOT_REQUIRE_LAYER_SHELL "Fail configuration when the optional LayerShellQt plugin cannot be built" OFF)
option(MARK_SHOT_WITH_LIBPORTAL "Enable libportal screencast negotiation when available" OFF)
option(MARK_SHOT_REQUIRE_LIBPORTAL "Fail configuration when libportal support is requested but unavailable" OFF)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets DBus Concurrent)
if(MARK_SHOT_WITH_LAYER_SHELL)
    find_package(LayerShellQt QUIET)
    if(MARK_SHOT_REQUIRE_LAYER_SHELL AND NOT LayerShellQt_FOUND)
        message(FATAL_ERROR "The optional LayerShellQt plugin was required but LayerShellQt was not found")
    endif()
endif()
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

if(COMMAND qt_standard_project_setup)
    qt_standard_project_setup()
else()
    set(CMAKE_AUTOMOC ON)
    set(CMAKE_AUTOUIC ON)
    set(CMAKE_AUTORCC ON)
endif()

qt_add_executable(mark-shot
    src/main.cpp
    src/clipboard_image.cpp
    src/clipboard_image.h
    src/debug_log.cpp
    src/debug_log.h
    src/capture_geometry.cpp
    src/capture_geometry.h
    src/layer_shell_plugin_interface.h
    src/layer_shell_runtime.cpp
    src/layer_shell_runtime.h
    src/window_detection.cpp
    src/window_detection.h
    src/screen_capture.cpp
    src/screen_capture.h
    src/shot_window.cpp
    src/shot_window.h
    src/annotation_launch.cpp
    src/annotation_launch.h
    src/scroll/stitcher.cpp
    src/scroll/stitcher.h
    src/scroll/scroll_session_window.cpp
    src/scroll/scroll_session_window.h
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

target_link_libraries(mark-shot
    PRIVATE
        Qt6::Core
        Qt6::Concurrent
        Qt6::DBus
        Qt6::Gui
        Qt6::Widgets
)

if(X11_xcb_FOUND)
    target_link_libraries(mark-shot PRIVATE X11::xcb)
    target_compile_definitions(mark-shot PRIVATE HAVE_XCB)
endif()

if(MARK_SHOT_WITH_LAYER_SHELL AND LayerShellQt_FOUND)
    qt_add_plugin(mark-shot-layer-shell MODULE
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
elseif(MARK_SHOT_WITH_LAYER_SHELL)
    message(STATUS "mark-shot: LayerShellQt not found, building without optional layer-shell plugin")
else()
    message(STATUS "mark-shot: optional layer-shell plugin disabled")
endif()

if(PipeWire_FOUND)
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
    message(STATUS "mark-shot: PipeWire not found, Wayland scrolling capture falls back to screenshot portal/grim")
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
endif()

install(TARGETS mark-shot RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
if(TARGET mark-shot-layer-shell)
    install(TARGETS mark-shot-layer-shell
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    )
endif()
install(PROGRAMS
    scripts/mark-shot-ocr
    scripts/mark-shot-translate
    scripts/mark-shot-window-detection-niri
    scripts/mark-shot-window-detection-hyprland
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

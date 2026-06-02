cmake_minimum_required(VERSION 3.20)

project(mark-shot VERSION 0.1.13 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_INSTALL_RPATH_USE_LINK_PATH ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets DBus Concurrent)
find_package(LayerShellQt QUIET)
find_package(X11 COMPONENTS xcb QUIET)
find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
    pkg_check_modules(PipeWire QUIET IMPORTED_TARGET libpipewire-0.3)
    pkg_check_modules(LibPortal QUIET IMPORTED_TARGET libportal)
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
    src/debug_log.cpp
    src/debug_log.h
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

if(LayerShellQt_FOUND)
    target_link_libraries(mark-shot PRIVATE LayerShellQt::Interface)
    target_compile_definitions(mark-shot PRIVATE HAVE_LAYER_SHELL)
endif()

if(PipeWire_FOUND)
    target_link_libraries(mark-shot PRIVATE PkgConfig::PipeWire)
    target_compile_definitions(mark-shot PRIVATE HAVE_PIPEWIRE)
    message(STATUS "mark-shot: PipeWire ${PipeWire_VERSION} found, portal screencast capture enabled")
    if(LibPortal_FOUND)
        target_link_libraries(mark-shot PRIVATE PkgConfig::LibPortal)
        target_compile_definitions(mark-shot PRIVATE HAVE_LIBPORTAL)
        message(STATUS "mark-shot: libportal ${LibPortal_VERSION} found, libportal screencast negotiation enabled")
    else()
        message(STATUS "mark-shot: libportal not found, using direct D-Bus portal screencast negotiation")
    endif()
else()
    message(STATUS "mark-shot: PipeWire not found, Wayland scrolling capture falls back to screenshot portal/grim")
endif()

install(TARGETS mark-shot RUNTIME DESTINATION bin)
install(PROGRAMS
    scripts/mark-shot-ocr
    scripts/mark-shot-translate
    DESTINATION bin
)
install(FILES
    data/mark-shot.desktop
    data/mark-shot-edit.desktop
    DESTINATION share/applications
)
install(FILES
    data/icons/hicolor/scalable/apps/mark-shot.svg
    data/icons/hicolor/scalable/apps/mark-shot-edit.svg
    DESTINATION share/icons/hicolor/scalable/apps
)

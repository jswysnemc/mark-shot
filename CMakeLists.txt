cmake_minimum_required(VERSION 3.20)

project(mark-shot VERSION 0.1.11 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_INSTALL_RPATH_USE_LINK_PATH ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets DBus Concurrent)
find_package(LayerShellQt QUIET)
find_package(X11 COMPONENTS xcb QUIET)
find_package(OpenCV QUIET COMPONENTS core imgproc features2d calib3d)

if(COMMAND qt_standard_project_setup)
    qt_standard_project_setup()
else()
    set(CMAKE_AUTOMOC ON)
    set(CMAKE_AUTOUIC ON)
    set(CMAKE_AUTORCC ON)
endif()

qt_add_executable(mark-shot
    src/main.cpp
    src/screen_capture.cpp
    src/screen_capture.h
    src/shot_window.cpp
    src/shot_window.h
    src/annotation_launch.cpp
    src/annotation_launch.h
    src/scroll/stitcher.cpp
    src/scroll/stitcher.h
    src/scroll/stitch_orb.cpp
    src/scroll/stitch_orb.h
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

if(OpenCV_FOUND)
    target_link_libraries(mark-shot PRIVATE ${OpenCV_LIBS})
    target_include_directories(mark-shot PRIVATE ${OpenCV_INCLUDE_DIRS})
    target_compile_definitions(mark-shot PRIVATE HAVE_OPENCV)
    message(STATUS "mark-shot: OpenCV ${OpenCV_VERSION} found, scrolling capture ORB path enabled")
else()
    message(STATUS "mark-shot: OpenCV not found, scrolling capture falls back to col-sample")
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

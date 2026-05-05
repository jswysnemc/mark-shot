cmake_minimum_required(VERSION 3.20)

project(mark-shot VERSION 0.1.4 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)
find_package(LayerShellQt REQUIRED)

qt_standard_project_setup()

qt_add_executable(mark-shot
    src/main.cpp
    src/screen_capture.cpp
    src/screen_capture.h
    src/shot_window.cpp
    src/shot_window.h
    src/ui/color_picker.cpp
    src/ui/color_picker.h
    src/ui/icons.cpp
    src/ui/icons.h
    src/ui/theme.cpp
    src/ui/theme.h
)

target_include_directories(mark-shot PRIVATE src)

target_link_libraries(mark-shot
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
        LayerShellQt::Interface
)

install(TARGETS mark-shot RUNTIME DESTINATION bin)
install(FILES data/mark-shot.desktop DESTINATION share/applications)
install(FILES data/icons/hicolor/scalable/apps/mark-shot.svg DESTINATION share/icons/hicolor/scalable/apps)

cmake_minimum_required(VERSION 3.20)
project(mark_shot_color_reference LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_AUTOMOC ON)
find_package(Qt6 REQUIRED COMPONENTS Widgets)
set(MARK_SHOT_QT_SOURCE "$ENV{HOME}/workspace/mark-shot/qt/src" CACHE PATH "Original Qt source directory")
add_executable(color-reference
    main.cpp config-fixture.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/color_picker.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/color_picker.h
    ${MARK_SHOT_QT_SOURCE}/ui/color_history_store.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/theme.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/toolbar_palette.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/toolbar_panel_style.cpp)
target_include_directories(color-reference PRIVATE ${MARK_SHOT_QT_SOURCE})
target_link_libraries(color-reference PRIVATE Qt6::Widgets)

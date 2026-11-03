cmake_minimum_required(VERSION 3.20)
project(mark_shot_text_reference LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
find_package(Qt6 REQUIRED COMPONENTS Widgets)
set(MARK_SHOT_QT_SOURCE "$ENV{HOME}/workspace/mark-shot/qt/src" CACHE PATH "Original Qt source directory")
add_executable(text-reference main.cpp
    ${MARK_SHOT_QT_SOURCE}/text_annotation_layout.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/theme.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/toolbar_palette.cpp
    ${MARK_SHOT_QT_SOURCE}/ui/toolbar_panel_style.cpp)
target_include_directories(text-reference PRIVATE
    ${MARK_SHOT_QT_SOURCE}
    ${MARK_SHOT_QT_SOURCE}/../bridge)
target_link_libraries(text-reference PRIVATE Qt6::Widgets)

cmake_minimum_required(VERSION 3.20)
project(region_detection_reference LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
find_package(Qt6 REQUIRED COMPONENTS Core Gui)
set(MARK_SHOT_QT_ROOT "/home/snemc/workspace/mark-shot/qt" CACHE PATH "Original screenshot sources")
set(REGION_ROOT "${MARK_SHOT_QT_ROOT}/src/region_detection")
add_executable(region-detection-reference
    reference.cpp
    "${REGION_ROOT}/image_gray.cpp"
    "${REGION_ROOT}/image_filters.cpp"
    "${REGION_ROOT}/image_components.cpp"
    "${REGION_ROOT}/region_detection.cpp"
    "${REGION_ROOT}/region_detector.cpp"
    "${REGION_ROOT}/region_edge_field.cpp"
    "${REGION_ROOT}/region_proposals.cpp"
    "${REGION_ROOT}/region_scoring.cpp"
    "${MARK_SHOT_QT_ROOT}/src/window_hover_selection.cpp")
target_include_directories(region-detection-reference PRIVATE "${MARK_SHOT_QT_ROOT}/src")
target_compile_options(region-detection-reference PRIVATE -O2)
target_link_libraries(region-detection-reference PRIVATE Qt6::Core Qt6::Gui)

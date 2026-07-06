cmake_minimum_required(VERSION 3.20)

project(mark-shot-sample-code-scan-plugin VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(Qt6 REQUIRED COMPONENTS Core Gui)

if(NOT DEFINED MARK_SHOT_PLUGIN_SDK_DIR)
    set(MARK_SHOT_PLUGIN_SDK_DIR "${CMAKE_CURRENT_LIST_DIR}/../../plugin-sdk")
endif()

if(NOT EXISTS "${MARK_SHOT_PLUGIN_SDK_DIR}/markshot/code_scan_provider_plugin.h")
    message(FATAL_ERROR "MARK_SHOT_PLUGIN_SDK_DIR must point to the plugin-sdk directory")
endif()

qt_standard_project_setup()

qt_add_plugin(mark-shot-sample-code-scan
    SHARED
    sample_code_scan_plugin.cpp
    sample_code_scan_plugin.h
)

target_include_directories(mark-shot-sample-code-scan PRIVATE "${MARK_SHOT_PLUGIN_SDK_DIR}")
target_link_libraries(mark-shot-sample-code-scan
    PRIVATE
        Qt6::Core
        Qt6::Gui
)

install(TARGETS mark-shot-sample-code-scan
        LIBRARY DESTINATION lib/mark-shot/plugins
        RUNTIME DESTINATION lib/mark-shot/plugins)

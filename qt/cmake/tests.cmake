qt_add_executable(mark-shot-color-history-store-test
    tests/color_history_store_test.cpp
    src/ui/color_history_store.cpp
    src/ui/color_history_store.h
    src/app_config_defaults.cpp
    src/app_config_defaults.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/window_detection.cpp
    src/window_detection_bundled_script.cpp
    src/window_detection.h
    src/config_value.cpp
    src/config_value.h
    src/debug_log.cpp
    src/debug_log.h
    src/shell_command.cpp
    src/shell_command.h
)
target_include_directories(mark-shot-color-history-store-test PRIVATE src)
target_link_libraries(mark-shot-color-history-store-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME color-history-store COMMAND mark-shot-color-history-store-test)

qt_add_executable(mark-shot-recent-font-store-test
    tests/recent_font_store_test.cpp
    src/ui/recent_font_store.cpp
    src/ui/recent_font_store.h
    src/app_config_defaults.cpp
    src/app_config_defaults.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/window_detection.cpp
    src/window_detection_bundled_script.cpp
    src/window_detection.h
    src/config_value.cpp
    src/config_value.h
    src/debug_log.cpp
    src/debug_log.h
    src/shell_command.cpp
    src/shell_command.h
)
target_include_directories(mark-shot-recent-font-store-test PRIVATE src)
target_link_libraries(mark-shot-recent-font-store-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME recent-font-store COMMAND mark-shot-recent-font-store-test)

qt_add_executable(mark-shot-selection-history-test
    tests/selection_history_test.cpp
    src/selection_history.cpp
    src/selection_history.h
    src/app_config_defaults.cpp
    src/app_config_defaults.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/window_detection.cpp
    src/window_detection_bundled_script.cpp
    src/window_detection.h
    src/config_value.cpp
    src/config_value.h
    src/debug_log.cpp
    src/debug_log.h
    src/shell_command.cpp
    src/shell_command.h
)
target_include_directories(mark-shot-selection-history-test PRIVATE src)
target_link_libraries(mark-shot-selection-history-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME selection-history COMMAND mark-shot-selection-history-test)

qt_add_executable(mark-shot-app-config-defaults-test
    tests/app_config_defaults_test.cpp
    src/app_config_defaults.cpp
    src/app_config_defaults.h
)
target_include_directories(mark-shot-app-config-defaults-test PRIVATE src)
target_link_libraries(mark-shot-app-config-defaults-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME app-config-defaults COMMAND mark-shot-app-config-defaults-test)

qt_add_executable(mark-shot-capture-session-screen-utils-test
    tests/capture_session_screen_utils_test.cpp
    src/capture_session_screen_utils.cpp
    src/capture_session_screen_utils.h
    src/debug_log.cpp
    src/debug_log.h
)
target_include_directories(mark-shot-capture-session-screen-utils-test PRIVATE src)
target_link_libraries(mark-shot-capture-session-screen-utils-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME capture-session-screen-utils COMMAND mark-shot-capture-session-screen-utils-test)

qt_add_executable(mark-shot-window-detection-session-test
    tests/window_detection_session_test.cpp
    src/window_detection_session.h
)
target_include_directories(mark-shot-window-detection-session-test PRIVATE src)
target_link_libraries(mark-shot-window-detection-session-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME window-detection-session COMMAND mark-shot-window-detection-session-test)

qt_add_executable(mark-shot-window-hover-selection-test
    tests/window_hover_selection_test.cpp
    src/window_hover_selection.cpp
    src/window_hover_selection.h
)
target_include_directories(mark-shot-window-hover-selection-test PRIVATE src)
target_link_libraries(mark-shot-window-hover-selection-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME window-hover-selection COMMAND mark-shot-window-hover-selection-test)

include(cmake/region_detection_sources.cmake)
qt_add_executable(mark-shot-region-detector-test
    tests/region_detector_test.cpp
    ${MARK_SHOT_REGION_DETECTION_SOURCES}
)
target_include_directories(mark-shot-region-detector-test PRIVATE src)
target_compile_definitions(mark-shot-region-detector-test
    PRIVATE
        MARK_SHOT_TEST_SOURCE_DIR="${CMAKE_CURRENT_SOURCE_DIR}"
)
target_link_libraries(mark-shot-region-detector-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME region-detector COMMAND mark-shot-region-detector-test)
include(cmake/region_detection_tests.cmake)
include(cmake/issue_regression_tests.cmake)

find_package(Python3 REQUIRED COMPONENTS Interpreter)
add_test(NAME packaging-configuration
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1
        ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/packaging_configuration_test.py
)
add_test(NAME niri-window-detection
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1
        ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/niri_window_detection_test.py
)
add_test(NAME kde-window-detection
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1
        ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/kde_window_detection_test.py
)
add_test(NAME rapidocr-model-type
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1
        ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/rapidocr_model_type_test.py
)

qt_add_executable(mark-shot-pinned-window-config-test
    tests/pinned_window_config_test.cpp
    src/pinned_window_config.cpp
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-pinned-window-config-test PRIVATE src)
target_link_libraries(mark-shot-pinned-window-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
        Qt6::Widgets
)
add_test(NAME pinned-window-config COMMAND mark-shot-pinned-window-config-test)

qt_add_executable(mark-shot-pinned-text-selection-metrics-test
    tests/pinned_text_selection_metrics_test.cpp
    src/pinned_window/pinned_text_selection_metrics.cpp
    src/pinned_window/pinned_text_selection_metrics.h
)
target_include_directories(mark-shot-pinned-text-selection-metrics-test PRIVATE src)
target_link_libraries(mark-shot-pinned-text-selection-metrics-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME pinned-text-selection-metrics COMMAND mark-shot-pinned-text-selection-metrics-test)

qt_add_executable(mark-shot-pinned-layer-shell-geometry-test
    tests/pinned_layer_shell_geometry_test.cpp
    src/pinned_window/pinned_layer_shell_geometry.cpp
    src/pinned_window/pinned_layer_shell_geometry.h
)
target_include_directories(mark-shot-pinned-layer-shell-geometry-test PRIVATE src)
target_link_libraries(mark-shot-pinned-layer-shell-geometry-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME pinned-layer-shell-geometry COMMAND mark-shot-pinned-layer-shell-geometry-test)

qt_add_executable(mark-shot-pinned-layer-shell-screen-binding-test
    tests/pinned_layer_shell_screen_binding_test.cpp
    src/pinned_window/pinned_layer_shell_geometry.cpp
    src/pinned_window/pinned_layer_shell_geometry.h
    src/pinned_window/pinned_layer_shell_screen_binding.cpp
    src/pinned_window/pinned_layer_shell_screen_binding.h
)
target_include_directories(mark-shot-pinned-layer-shell-screen-binding-test PRIVATE src)
target_link_libraries(mark-shot-pinned-layer-shell-screen-binding-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME pinned-layer-shell-screen-binding
    COMMAND mark-shot-pinned-layer-shell-screen-binding-test
)

qt_add_executable(mark-shot-pinned-resize-controller-test
    tests/pinned_resize_controller_test.cpp
    src/pinned_window/pinned_native_resize.cpp
    src/pinned_window/pinned_native_resize.h
    src/pinned_window/pinned_resize_controller.cpp
    src/pinned_window/pinned_resize_controller.h
)
target_include_directories(mark-shot-pinned-resize-controller-test PRIVATE src)
target_link_libraries(mark-shot-pinned-resize-controller-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
        Qt6::Test
)
add_test(NAME pinned-resize-controller COMMAND mark-shot-pinned-resize-controller-test)

qt_add_executable(mark-shot-stitcher-test
    tests/stitcher_test.cpp
    src/debug_log.cpp
    src/debug_log.h
    src/scroll/stitcher.cpp
    src/scroll/stitcher.h
    src/scroll/stitcher_fixed_regions.cpp
    src/scroll/stitcher_fixed_regions.h
)
target_include_directories(mark-shot-stitcher-test PRIVATE src)
target_link_libraries(mark-shot-stitcher-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME stitcher COMMAND mark-shot-stitcher-test)

qt_add_executable(mark-shot-stitcher-sidebar-test
    tests/stitcher_sidebar_test.cpp
    "$<FILTER:$<TARGET_OBJECTS:mark-shot-stitcher-test>,EXCLUDE,/(tests/stitcher_test\\.cpp|mocs_compilation\\.cpp)\\.(o|obj)$>"
)
target_include_directories(mark-shot-stitcher-sidebar-test PRIVATE src)
target_link_libraries(mark-shot-stitcher-sidebar-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_dependencies(mark-shot-stitcher-sidebar-test mark-shot-stitcher-test)
add_test(NAME stitcher-sidebar COMMAND mark-shot-stitcher-sidebar-test)

qt_add_executable(mark-shot-stitcher-fixed-regions-test
    tests/stitcher_fixed_regions_test.cpp
    src/debug_log.cpp
    src/debug_log.h
    src/scroll/stitcher.cpp
    src/scroll/stitcher.h
    src/scroll/stitcher_fixed_regions.cpp
    src/scroll/stitcher_fixed_regions.h
)
target_include_directories(mark-shot-stitcher-fixed-regions-test PRIVATE src)
target_link_libraries(mark-shot-stitcher-fixed-regions-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME stitcher-fixed-regions COMMAND mark-shot-stitcher-fixed-regions-test)

qt_add_executable(mark-shot-clipboard-publish-policy-test
    tests/clipboard_publish_policy_test.cpp
    src/clipboard_publish_policy.cpp
    src/clipboard_publish_policy.h
)
target_include_directories(mark-shot-clipboard-publish-policy-test PRIVATE src)
target_link_libraries(mark-shot-clipboard-publish-policy-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME clipboard-publish-policy COMMAND mark-shot-clipboard-publish-policy-test)

qt_add_executable(mark-shot-selection-cursor-nudge-test
    tests/selection_cursor_nudge_test.cpp
    src/selection_cursor_nudge.cpp
    src/selection_cursor_nudge.h
)
target_include_directories(mark-shot-selection-cursor-nudge-test PRIVATE src)
target_link_libraries(mark-shot-selection-cursor-nudge-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME selection-cursor-nudge COMMAND mark-shot-selection-cursor-nudge-test)

qt_add_executable(mark-shot-selection-loupe-test
    tests/selection_loupe_test.cpp
    src/capture_cross_cursor.cpp
    src/selection_loupe.cpp
    src/selection_loupe.h
)
target_include_directories(mark-shot-selection-loupe-test PRIVATE src)
target_link_libraries(mark-shot-selection-loupe-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME selection-loupe COMMAND mark-shot-selection-loupe-test)

qt_add_executable(mark-shot-pinned-kde-keep-above-test
    tests/pinned_kde_keep_above_test.cpp
    src/pinned_window/pinned_kde_keep_above_script.cpp
    src/pinned_window/pinned_kde_keep_above_script.h
)
target_include_directories(mark-shot-pinned-kde-keep-above-test PRIVATE src)
target_link_libraries(mark-shot-pinned-kde-keep-above-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME pinned-kde-keep-above COMMAND mark-shot-pinned-kde-keep-above-test)
find_package(Qt6 QUIET COMPONENTS Qml)
if(TARGET Qt6::Qml)
    target_link_libraries(mark-shot-pinned-kde-keep-above-test PRIVATE Qt6::Qml)
    target_compile_definitions(mark-shot-pinned-kde-keep-above-test PRIVATE MARK_SHOT_TEST_WITH_QML=1)
endif()

qt_add_executable(mark-shot-selection-loupe-config-test
    tests/selection_loupe_config_test.cpp
    src/selection_loupe_config.cpp
    src/selection_loupe_config.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-selection-loupe-config-test PRIVATE src)
target_link_libraries(mark-shot-selection-loupe-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME selection-loupe-config COMMAND mark-shot-selection-loupe-config-test)

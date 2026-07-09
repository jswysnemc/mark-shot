qt_add_executable(mark-shot-color-history-store-test
    tests/color_history_store_test.cpp
    src/ui/color_history_store.cpp
    src/ui/color_history_store.h
    src/app_config_defaults.cpp
    src/app_config_defaults.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/window_detection.cpp
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

find_package(Python3 REQUIRED COMPONENTS Interpreter)
add_test(NAME niri-window-detection
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1
        ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/niri_window_detection_test.py
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
    src/pinned_window/pinned_resize_controller.cpp
    src/pinned_window/pinned_resize_controller.h
)
target_include_directories(mark-shot-pinned-resize-controller-test PRIVATE src)
target_link_libraries(mark-shot-pinned-resize-controller-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME pinned-resize-controller COMMAND mark-shot-pinned-resize-controller-test)

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

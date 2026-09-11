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

qt_add_executable(mark-shot-line-skeleton-path-test
    tests/line_skeleton_path_test.cpp
    src/line_skeleton_path.cpp
    src/line_skeleton_path.h
)
target_include_directories(mark-shot-line-skeleton-path-test PRIVATE src)
target_link_libraries(mark-shot-line-skeleton-path-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME line-skeleton-path COMMAND mark-shot-line-skeleton-path-test)

qt_add_executable(mark-shot-config-value-test
    tests/config_value_test.cpp
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-config-value-test PRIVATE src)
target_link_libraries(mark-shot-config-value-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME config-value COMMAND mark-shot-config-value-test)

include(cmake/recording_tests.cmake)
include(cmake/capture_tests.cmake)
include(cmake/capture_history_tests.cmake)
include(cmake/shot_window_tests.cmake)
include(cmake/debug_tests.cmake)
include(cmake/provider_tests.cmake)
include(cmake/marketplace_tests.cmake)
include(cmake/settings_tests.cmake)

qt_add_executable(mark-shot-startup-default-color-test
    tests/startup_default_color_test.cpp
)
target_include_directories(mark-shot-startup-default-color-test PRIVATE src)
target_link_libraries(mark-shot-startup-default-color-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
        Qt6::Test
)
add_test(NAME startup-default-color COMMAND mark-shot-startup-default-color-test)

qt_add_executable(mark-shot-toolbar-appearance-config-test
    tests/toolbar_appearance_config_test.cpp
    src/toolbar_appearance_config.cpp
    src/toolbar_appearance_config.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-toolbar-appearance-config-test PRIVATE src)
target_link_libraries(mark-shot-toolbar-appearance-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME toolbar-appearance-config COMMAND mark-shot-toolbar-appearance-config-test)

qt_add_executable(mark-shot-clipboard-image-config-test
    tests/clipboard_image_config_test.cpp
    src/clipboard_image_config.cpp
    src/clipboard_image_config.h
    src/app_config_defaults.cpp
    src/app_config_defaults.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/config_value.cpp
    src/config_value.h
    src/debug_log.cpp
    src/debug_log.h
    src/shell_command.cpp
    src/shell_command.h
    src/window_detection.cpp
    src/window_detection.h
)
target_include_directories(mark-shot-clipboard-image-config-test PRIVATE src)
target_link_libraries(mark-shot-clipboard-image-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME clipboard-image-config COMMAND mark-shot-clipboard-image-config-test)

qt_add_executable(mark-shot-export-image-effect-test
    tests/export_image_effect_test.cpp
    src/export_image_effect.cpp
    src/export_image_effect.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-export-image-effect-test PRIVATE src)
target_link_libraries(mark-shot-export-image-effect-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
        Qt6::Test
)
add_test(NAME export-image-effect COMMAND mark-shot-export-image-effect-test)

qt_add_executable(mark-shot-line-constraint-test
    tests/shot_window_line_constraint_test.cpp
    src/shot_window_line_constraint.cpp
    src/shot_window_line_constraint.h
)
target_include_directories(mark-shot-line-constraint-test PRIVATE src)
target_link_libraries(mark-shot-line-constraint-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME line-constraint COMMAND mark-shot-line-constraint-test)

qt_add_executable(mark-shot-capture-freeze-scope-test
    tests/capture_freeze_scope_test.cpp
    src/capture_freeze_scope.cpp
    src/capture_freeze_scope.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-capture-freeze-scope-test PRIVATE src)
target_link_libraries(mark-shot-capture-freeze-scope-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME capture-freeze-scope COMMAND mark-shot-capture-freeze-scope-test)

qt_add_executable(mark-shot-capture-double-click-action-test
    tests/capture_double_click_action_test.cpp
    src/capture_double_click_action.cpp
    src/capture_double_click_action.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-capture-double-click-action-test PRIVATE src)
target_link_libraries(mark-shot-capture-double-click-action-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME capture-double-click-action COMMAND mark-shot-capture-double-click-action-test)

qt_add_executable(mark-shot-capture-cursor-policy-test
    tests/capture_cursor_policy_test.cpp
    src/capture_cursor_policy.cpp
    src/capture_cursor_policy.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-capture-cursor-policy-test PRIVATE src)
target_link_libraries(mark-shot-capture-cursor-policy-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME capture-cursor-policy COMMAND mark-shot-capture-cursor-policy-test)

qt_add_executable(mark-shot-capture-own-windows-policy-test
    tests/capture_own_windows_policy_test.cpp
    src/capture_own_windows_policy.cpp
    src/capture_own_windows_policy.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-capture-own-windows-policy-test PRIVATE src)
target_link_libraries(mark-shot-capture-own-windows-policy-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME capture-own-windows-policy COMMAND mark-shot-capture-own-windows-policy-test)

qt_add_executable(mark-shot-kde-capture-config-test
    tests/kde_capture_config_test.cpp
    src/kde_capture_config.cpp
    src/kde_capture_config.h
    src/app_config_defaults.cpp
    src/app_config_defaults.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/config_value.cpp
    src/config_value.h
    src/debug_log.cpp
    src/debug_log.h
    src/shell_command.cpp
    src/shell_command.h
    src/window_detection.cpp
    src/window_detection.h
)
target_include_directories(mark-shot-kde-capture-config-test PRIVATE src)
target_link_libraries(mark-shot-kde-capture-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME kde-capture-config COMMAND mark-shot-kde-capture-config-test)

qt_add_executable(mark-shot-save-path-config-test
    tests/save_path_config_test.cpp
    src/save_path_config.cpp
    src/save_path_config.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-save-path-config-test PRIVATE src)
target_link_libraries(mark-shot-save-path-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME save-path-config COMMAND mark-shot-save-path-config-test)

qt_add_executable(mark-shot-screen-capture-cursor-test
    tests/screen_capture_cursor_test.cpp
    src/screen_capture_cursor.cpp
    src/screen_capture_cursor.h
    src/screen_capture.h
)
target_include_directories(mark-shot-screen-capture-cursor-test PRIVATE src)
target_link_libraries(mark-shot-screen-capture-cursor-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME screen-capture-cursor COMMAND mark-shot-screen-capture-cursor-test)

qt_add_executable(mark-shot-ocr-result-test
    tests/ocr_result_test.cpp
    src/ocr_result.cpp
    src/ocr_result.h
)
target_include_directories(mark-shot-ocr-result-test PRIVATE src)
target_link_libraries(mark-shot-ocr-result-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME ocr-result COMMAND mark-shot-ocr-result-test)

qt_add_executable(mark-shot-code-scan-result-test
    tests/code_scan_result_test.cpp
    src/code_scan_result.cpp
    src/code_scan_result.h
)
target_include_directories(mark-shot-code-scan-result-test PRIVATE src)
target_link_libraries(mark-shot-code-scan-result-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME code-scan-result COMMAND mark-shot-code-scan-result-test)

qt_add_executable(mark-shot-ui-language-config-test
    tests/ui_language_config_test.cpp
    src/ui/interface_language_config.cpp
    src/ui/interface_language_config.h
)
target_include_directories(mark-shot-ui-language-config-test PRIVATE src)
target_link_libraries(mark-shot-ui-language-config-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME ui-language-config COMMAND mark-shot-ui-language-config-test)

qt_add_executable(mark-shot-ui-theme-config-test
    tests/ui_theme_config_test.cpp
    src/ui/interface_theme_config.cpp
    src/ui/interface_theme_config.h
    src/ui/theme.cpp
    src/ui/theme.h
)
target_include_directories(mark-shot-ui-theme-config-test PRIVATE src)
target_link_libraries(mark-shot-ui-theme-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
        Qt6::Test
)
add_test(NAME ui-theme-config COMMAND mark-shot-ui-theme-config-test)

qt_add_executable(mark-shot-global-shortcut-keymap-test
    tests/global_shortcut_keymap_test.cpp
    src/shortcuts/global_shortcut_keymap.cpp
    src/shortcuts/global_shortcut_keymap.h
)
target_include_directories(mark-shot-global-shortcut-keymap-test PRIVATE src)
target_link_libraries(mark-shot-global-shortcut-keymap-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME global-shortcut-keymap COMMAND mark-shot-global-shortcut-keymap-test)

include(cmake/tests.cmake)
include(cmake/release_tests.cmake)

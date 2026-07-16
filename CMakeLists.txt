cmake_minimum_required(VERSION 3.20)

project(mark-shot VERSION 0.1.40 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_INSTALL_RPATH_USE_LINK_PATH ON)

include(GNUInstallDirs)

option(MARK_SHOT_WITH_LAYER_SHELL "Build the optional LayerShellQt plugin when available" ON)
option(MARK_SHOT_REQUIRE_LAYER_SHELL "Fail configuration when the optional LayerShellQt plugin cannot be built" OFF)
option(MARK_SHOT_WITH_LIBPORTAL "Enable libportal screencast negotiation when available" OFF)
option(MARK_SHOT_REQUIRE_LIBPORTAL "Fail configuration when libportal support is requested but unavailable" OFF)
option(MARK_SHOT_REQUIRE_FFMPEG "Fail configuration when FFmpeg libav recording support is unavailable" OFF)

if(WIN32)
    enable_language(RC)
endif()

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(MARK_SHOT_LINUX TRUE)
else()
    set(MARK_SHOT_LINUX FALSE)
endif()

set(MARK_SHOT_QT_COMPONENTS Core Gui Widgets Concurrent Network)
if(MARK_SHOT_LINUX)
    list(APPEND MARK_SHOT_QT_COMPONENTS DBus)
endif()
find_package(Qt6 REQUIRED COMPONENTS ${MARK_SHOT_QT_COMPONENTS})

if(MARK_SHOT_WITH_LAYER_SHELL AND MARK_SHOT_LINUX)
    find_package(LayerShellQt QUIET)
    if(MARK_SHOT_REQUIRE_LAYER_SHELL AND NOT LayerShellQt_FOUND)
        message(FATAL_ERROR "The optional LayerShellQt plugin was required but LayerShellQt was not found")
    endif()
elseif(MARK_SHOT_REQUIRE_LAYER_SHELL)
    message(FATAL_ERROR "The optional LayerShellQt plugin is only supported on Linux")
endif()
if(MARK_SHOT_LINUX)
    find_package(X11 COMPONENTS xcb QUIET)
    find_package(PkgConfig QUIET)
    if(PkgConfig_FOUND)
        pkg_check_modules(PipeWire QUIET IMPORTED_TARGET libpipewire-0.3)
        pkg_check_modules(WaylandClient QUIET IMPORTED_TARGET wayland-client)
        pkg_check_modules(PulseAudioRecording QUIET IMPORTED_TARGET
            libpulse-simple
            libpulse
        )
        if(MARK_SHOT_WITH_LIBPORTAL)
            pkg_check_modules(LibPortal QUIET IMPORTED_TARGET libportal)
            if(MARK_SHOT_REQUIRE_LIBPORTAL AND NOT LibPortal_FOUND)
                message(FATAL_ERROR "libportal support was required but libportal was not found")
            endif()
        endif()
    endif()
    find_program(WAYLAND_SCANNER_EXECUTABLE NAMES wayland-scanner)
elseif(MARK_SHOT_REQUIRE_LIBPORTAL)
    message(FATAL_ERROR "libportal support is only supported on Linux")
endif()

include(cmake/zxing.cmake)

include(cmake/ffmpeg_libav.cmake)

if(COMMAND qt_standard_project_setup)
    qt_standard_project_setup()
    # qt_standard_project_setup() enables AUTOMOC/AUTOUIC but NOT AUTORCC, so a
    # .qrc listed as a target source would be silently dropped (resources never
    # embedded). Enable it explicitly so resources.qrc is compiled in.
    set(CMAKE_AUTORCC ON)
else()
    set(CMAKE_AUTOMOC ON)
    set(CMAKE_AUTOUIC ON)
    set(CMAKE_AUTORCC ON)
endif()

include(cmake/ipc_sources.cmake)
include(cmake/cli_sources.cmake)
include(cmake/notification_sources.cmake)
include(cmake/pipewire_capture_sources.cmake)
include(cmake/provider_sources.cmake)
include(cmake/marketplace_sources.cmake)
include(cmake/recording_sources.cmake)
include(cmake/settings_sources.cmake)
include(cmake/shot_window_sources.cmake)

qt_add_executable(mark-shot
    resources.qrc
    ${MARK_SHOT_IPC_SOURCES}
    ${MARK_SHOT_CLI_SOURCES}
    ${MARK_SHOT_NOTIFICATION_SOURCES}
    ${MARK_SHOT_PROVIDER_SOURCES}
    ${MARK_SHOT_MARKETPLACE_SOURCES}
    ${MARK_SHOT_RECORDING_SOURCES}
    ${MARK_SHOT_SETTINGS_SOURCES}
    ${MARK_SHOT_WINDOW_SOURCES}
    src/main.cpp
    src/clipboard_image.cpp
    src/clipboard_image.h
    src/clipboard_image_config.cpp
    src/clipboard_image_config.h
    src/code_scan_result.cpp
    src/code_scan_result.h
    src/cli/image_pin_launch.cpp
    src/cli/image_pin_launch.h
    src/app_config_defaults.cpp
    src/app_config_defaults.h
    src/app_config_store.cpp
    src/app_config_store.h
    src/debug_log.cpp
    src/debug_log.h
    src/export_image_effect.cpp
    src/export_image_effect.h
    src/config_value.cpp
    src/config_value.h
    src/toolbar_appearance_config.cpp
    src/toolbar_appearance_config.h
    src/translation_language_options.cpp
    src/translation_language_options.h
    src/capture_cursor_policy.cpp
    src/capture_cursor_policy.h
    src/capture_own_windows_policy.cpp
    src/capture_own_windows_policy.h
    src/capture_own_windows_policy_config.cpp
    src/display_capture/display_capture_card.cpp
    src/display_capture/display_capture_card.h
    src/display_capture/display_capture_picker.cpp
    src/display_capture/display_capture_picker.h
    src/display_capture/display_capture_save.cpp
    src/display_capture/display_capture_save.h
    src/display_capture/display_capture_snapshot.cpp
    src/display_capture/display_capture_snapshot.h
    src/display_capture/display_capture_target.h
    src/capture_cursor_policy_config.cpp
    src/capture_freeze_scope.cpp
    src/capture_freeze_scope.h
    src/capture_freeze_scope_config.cpp
    src/kde_capture_config.cpp
    src/kde_capture_config.h
    src/capture_geometry.cpp
    src/capture_geometry.h
    src/capture_session_launcher.cpp
    src/capture_session_launcher.h
    src/layer_shell_plugin_interface.h
    src/layer_shell_runtime.cpp
    src/layer_shell_runtime.h
    src/line_skeleton_path.cpp
    src/line_skeleton_path.h
    src/autostart/autostart_linux.cpp
    src/autostart/autostart_manager.cpp
    src/autostart/autostart_manager.h
    src/autostart/autostart_windows.cpp
    src/startup_config.cpp
    src/startup_config.h
    src/startup_shortcut_hint.cpp
    src/startup_shortcut_hint.h
    src/window_detection.cpp
    src/window_detection.h
    src/windows_integration.cpp
    src/windows_integration.h
    src/windows_tray_controller.cpp
    src/windows_tray_controller.h
    src/screen_capture.cpp
    src/screen_capture.h
    src/screen_capture_api.cpp
    src/screen_capture_core_helpers.cpp
    src/screen_capture_cursor.cpp
    src/screen_capture_cursor.h
    src/screen_capture_dbus_portal.cpp
    src/screen_capture_dbus_wayland.cpp
    src/screen_capture_internal.h
    ${MARK_SHOT_PIPEWIRE_CAPTURE_SOURCES}
    src/screen_capture_platform_api.cpp
    src/screen_capture_windows_wgc.cpp
    src/screen_capture_x11.cpp
    src/shell_command.cpp
    src/shell_command.h
    src/ocr_result.cpp
    src/ocr_result.h
    src/save_path_config.cpp
    src/save_path_config.h
    src/shortcut_config.cpp
    src/shortcut_config.h
    src/pinned_window_config.cpp
    src/pinned_window/pinned_image_window.cpp
    src/pinned_window/pinned_image_window.h
    src/pinned_window/pinned_image_window_geometry.cpp
    src/pinned_window/pinned_image_window_layer_shell.cpp
    src/pinned_window/pinned_image_window_ocr.cpp
    src/pinned_window/pinned_image_window_text_selection.cpp
    src/pinned_window/pinned_image_window_translation.cpp
    src/pinned_window/pinned_layer_shell_geometry.cpp
    src/pinned_window/pinned_layer_shell_geometry.h
    src/pinned_window/pinned_layer_shell_screen_binding.cpp
    src/pinned_window/pinned_layer_shell_screen_binding.h
    src/pinned_window/pinned_resize_controller.cpp
    src/pinned_window/pinned_resize_controller.h
    src/pinned_window/pinned_text_selection_metrics.cpp
    src/pinned_window/pinned_text_selection_metrics.h
    src/pinned_window_top.cpp
    src/pinned_window_top.h
    src/annotation_launch.cpp
    src/annotation_launch.h
    src/scroll/stitcher.cpp
    src/scroll/stitcher.h
    src/scroll/stitcher_internal.cpp
    src/scroll/stitcher_internal.h
    src/scroll/stitcher_algorithm.cpp
    src/scroll/scroll_session_window.cpp
    src/scroll/scroll_session_window_capture.cpp
    src/scroll/scroll_session_window_events.cpp
    src/scroll/scroll_session_window.h
    src/scroll/scroll_session_window_internal.h
    src/scroll/scroll_session_window_lifecycle.cpp
    src/scroll/scroll_session_window_preview.cpp
    src/ui/color_picker.cpp
    src/ui/color_picker.h
    src/ui/color_history_store.cpp
    src/ui/color_history_store.h
    src/ui/icons.cpp
    src/ui/icons.h
    src/ui/i18n.cpp
    src/ui/i18n.h
    src/ui/interface_language_config.cpp
    src/ui/interface_language_config.h
    src/ui/interface_theme_config.cpp
    src/ui/interface_theme_config.h
    src/ui/theme.cpp
    src/ui/theme.h
)

include(cmake/capture_session_sources.cmake)

target_include_directories(mark-shot PRIVATE src plugin-sdk)
target_compile_definitions(mark-shot PRIVATE MARK_SHOT_VERSION="${PROJECT_VERSION}")
if(WIN32)
    set_property(TARGET mark-shot PROPERTY CXX_STANDARD 20)
    set(MARK_SHOT_WINDOWS_ICON "${CMAKE_CURRENT_SOURCE_DIR}/packaging/windows/mark-shot.ico")
    configure_file(packaging/windows/mark-shot.rc.in "${CMAKE_CURRENT_BINARY_DIR}/mark-shot.rc" @ONLY)
    target_sources(mark-shot PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/mark-shot.rc")
    set_target_properties(mark-shot PROPERTIES WIN32_EXECUTABLE TRUE)
    target_link_libraries(mark-shot PRIVATE d3d11 dwmapi dxgi ksuser ole32 user32 uuid windowsapp)
endif()

target_link_libraries(mark-shot
    PRIVATE
        Qt6::Core
        Qt6::Concurrent
        Qt6::Gui
        Qt6::Network
        Qt6::Widgets
)

if(ZXing_FOUND)
    target_link_libraries(mark-shot PRIVATE ZXing::ZXing)
    target_compile_definitions(mark-shot PRIVATE HAVE_ZXING_CPP)
    message(STATUS "mark-shot: builtin zxing-cpp code scanner enabled (CMake package)")
elseif(ZXingCpp_FOUND)
    target_link_libraries(mark-shot PRIVATE PkgConfig::ZXingCpp)
    target_compile_definitions(mark-shot PRIVATE HAVE_ZXING_CPP)
    message(STATUS "mark-shot: builtin zxing-cpp code scanner enabled (pkg-config)")
else()
    message(STATUS "mark-shot: builtin zxing-cpp code scanner disabled")
endif()

if(MARK_SHOT_LINUX)
    target_sources(mark-shot
        PRIVATE
            src/global_shortcut_portal.cpp
            src/global_shortcut_portal.h
    )
    target_link_libraries(mark-shot PRIVATE Qt6::DBus)
    target_compile_definitions(mark-shot PRIVATE MARK_SHOT_WITH_DBUS)
endif()

if(MARK_SHOT_LINUX AND X11_xcb_FOUND)
    target_link_libraries(mark-shot PRIVATE X11::xcb)
    target_compile_definitions(mark-shot PRIVATE HAVE_XCB)
endif()

if(FFmpegLibav_FOUND)
    target_link_libraries(mark-shot PRIVATE MarkShot::FFmpegLibav)
    target_compile_definitions(mark-shot PRIVATE HAVE_LIBAV_RECORDING)
    message(STATUS "mark-shot: FFmpeg libav recording writer enabled")
else()
    message(STATUS "mark-shot: FFmpeg libav recording writer disabled")
endif()
if(PulseAudioRecording_FOUND)
    target_link_libraries(mark-shot PRIVATE PkgConfig::PulseAudioRecording)
    target_compile_definitions(mark-shot PRIVATE HAVE_PULSE_RECORDING)
    message(STATUS "mark-shot: PulseAudio lib recording capture enabled")
else()
    message(STATUS "mark-shot: PulseAudio lib recording capture disabled")
endif()

if(MARK_SHOT_WITH_LAYER_SHELL AND MARK_SHOT_LINUX AND LayerShellQt_FOUND)
    qt_add_plugin(mark-shot-layer-shell MODULE)
    target_sources(mark-shot-layer-shell
        PRIVATE
        src/debug_log.cpp
        src/debug_log.h
        src/layer_shell_plugin.cpp
        src/layer_shell_plugin_interface.h
    )
    target_include_directories(mark-shot-layer-shell PRIVATE src)
    target_link_libraries(mark-shot-layer-shell
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Widgets
            LayerShellQt::Interface
    )
    set_target_properties(mark-shot-layer-shell PROPERTIES
        LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}"
    )
    message(STATUS "mark-shot: LayerShellQt found, optional layer-shell plugin enabled")
elseif(MARK_SHOT_WITH_LAYER_SHELL AND MARK_SHOT_LINUX)
    message(STATUS "mark-shot: LayerShellQt not found, building without optional layer-shell plugin")
elseif(MARK_SHOT_WITH_LAYER_SHELL)
    message(STATUS "mark-shot: LayerShellQt is Linux-only, building without optional layer-shell plugin")
else()
    message(STATUS "mark-shot: optional layer-shell plugin disabled")
endif()

if(MARK_SHOT_LINUX AND WaylandClient_FOUND AND WAYLAND_SCANNER_EXECUTABLE)
    set(MARK_SHOT_WAYLAND_GENERATED_DIR "${CMAKE_CURRENT_BINARY_DIR}/generated/wayland")
    set(MARK_SHOT_WLR_SCREENCOPY_XML "${CMAKE_CURRENT_SOURCE_DIR}/protocols/wlr-screencopy-unstable-v1.xml")
    set(MARK_SHOT_WLR_SCREENCOPY_HEADER
        "${MARK_SHOT_WAYLAND_GENERATED_DIR}/wlr-screencopy-unstable-v1-client-protocol.h")
    set(MARK_SHOT_WLR_SCREENCOPY_CODE
        "${MARK_SHOT_WAYLAND_GENERATED_DIR}/wlr-screencopy-unstable-v1-protocol.c")
    add_custom_command(
        OUTPUT
            "${MARK_SHOT_WLR_SCREENCOPY_HEADER}"
            "${MARK_SHOT_WLR_SCREENCOPY_CODE}"
        COMMAND "${CMAKE_COMMAND}" -E make_directory "${MARK_SHOT_WAYLAND_GENERATED_DIR}"
        COMMAND "${WAYLAND_SCANNER_EXECUTABLE}" client-header
            "${MARK_SHOT_WLR_SCREENCOPY_XML}"
            "${MARK_SHOT_WLR_SCREENCOPY_HEADER}"
        COMMAND "${WAYLAND_SCANNER_EXECUTABLE}" private-code
            "${MARK_SHOT_WLR_SCREENCOPY_XML}"
            "${MARK_SHOT_WLR_SCREENCOPY_CODE}"
        DEPENDS "${MARK_SHOT_WLR_SCREENCOPY_XML}"
        VERBATIM
    )
    target_sources(mark-shot PRIVATE "${MARK_SHOT_WLR_SCREENCOPY_CODE}")
    target_include_directories(mark-shot PRIVATE "${MARK_SHOT_WAYLAND_GENERATED_DIR}")
    target_link_libraries(mark-shot PRIVATE PkgConfig::WaylandClient)
    target_compile_definitions(mark-shot PRIVATE HAVE_WLROOTS_SCREENCOPY)
    message(STATUS "mark-shot: wlroots screencopy recording backend enabled")
elseif(MARK_SHOT_LINUX)
    message(STATUS "mark-shot: wlroots screencopy recording backend disabled")
endif()

include(cmake/pipewire_capture_target.cmake)

# provider 插件：可选构建，依赖缺失时各自跳过；需在测试注册前定义 target
add_subdirectory(plugins/ocr-rapid)
add_subdirectory(plugins/translate-openai)
add_subdirectory(plugins/code-scan-zxing)

include(CTest)
if(BUILD_TESTING)
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

    include(cmake/tests.cmake)
endif()

include(cmake/install_rules.cmake)

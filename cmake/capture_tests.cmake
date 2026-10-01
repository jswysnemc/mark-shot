qt_add_executable(mark-shot-capture-own-windows-guard-test
    tests/capture_own_windows_guard_test.cpp
    src/capture_own_windows_guard.cpp
    src/capture_own_windows_guard.h
    src/debug_log.cpp
    src/debug_log.h
)
target_include_directories(mark-shot-capture-own-windows-guard-test PRIVATE src)
target_link_libraries(mark-shot-capture-own-windows-guard-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
        Qt6::Widgets
)
add_test(NAME capture-own-windows-guard COMMAND mark-shot-capture-own-windows-guard-test)

qt_add_executable(mark-shot-capture-delay-test
    tests/capture_delay_test.cpp
    src/capture_delay/capture_delay_option.cpp
    src/capture_delay/capture_delay_option.h
    src/capture_delay/capture_delay_scheduler.cpp
    src/capture_delay/capture_delay_scheduler.h
    src/debug_log.cpp
    src/debug_log.h
)
target_include_directories(mark-shot-capture-delay-test PRIVATE src)
target_link_libraries(mark-shot-capture-delay-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME capture-delay COMMAND mark-shot-capture-delay-test)

if(MARK_SHOT_LINUX)
    qt_add_executable(mark-shot-screen-capture-wayland-routing-test
        tests/screen_capture_wayland_routing_test.cpp
        src/screen_capture_wayland_routing.cpp
        src/capture_own_windows_guard.cpp
        src/capture_own_windows_policy.cpp
        src/config_value.cpp
        src/debug_log.cpp
        src/recording/recording_capture_stream.h
        src/recording/recording_polling_capture_stream.cpp
        src/recording/recording_polling_capture_stream.h
    )
    target_include_directories(mark-shot-screen-capture-wayland-routing-test PRIVATE src)
    target_compile_definitions(mark-shot-screen-capture-wayland-routing-test PRIVATE MARK_SHOT_WITH_DBUS=1)
    target_link_libraries(mark-shot-screen-capture-wayland-routing-test
        PRIVATE Qt6::Core Qt6::Gui Qt6::Widgets Qt6::DBus Qt6::Test)
    add_test(NAME screen-capture-wayland-routing COMMAND mark-shot-screen-capture-wayland-routing-test)
endif()

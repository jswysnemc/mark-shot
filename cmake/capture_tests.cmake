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

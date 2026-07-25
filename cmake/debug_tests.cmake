qt_add_executable(mark-shot-debug-log-test
    tests/debug_log_test.cpp
    src/debug_log.cpp
    src/debug_log.h
)
target_include_directories(mark-shot-debug-log-test PRIVATE src)
target_link_libraries(mark-shot-debug-log-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME debug-log COMMAND mark-shot-debug-log-test)

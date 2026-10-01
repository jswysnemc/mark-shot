qt_add_executable(mark-shot-single-instance-ipc-test
    tests/single_instance_ipc_test.cpp
    src/ipc/single_instance_ipc.cpp
    src/ipc/single_instance_ipc.h
    src/recording/recording_status.cpp
    src/recording/recording_status.h
    src/debug_log.cpp
    src/debug_log.h
)
target_include_directories(mark-shot-single-instance-ipc-test PRIVATE src)
target_link_libraries(mark-shot-single-instance-ipc-test
    PRIVATE Qt6::Core Qt6::Gui Qt6::Network Qt6::Test)
add_test(NAME single-instance-ipc COMMAND mark-shot-single-instance-ipc-test)

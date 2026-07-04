qt_add_executable(mark-shot-recording-capture-backend-test
    tests/recording_capture_backend_test.cpp
    src/recording/recording_capture_backend.cpp
    src/recording/recording_capture_backend.h
)
target_include_directories(mark-shot-recording-capture-backend-test PRIVATE src)
target_link_libraries(mark-shot-recording-capture-backend-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME recording-capture-backend COMMAND mark-shot-recording-capture-backend-test)

if(TARGET mark-shot-recording-libav)
    add_library(mark-shot-recording-bad-abi MODULE tests/fixtures/recording_backend_bad_abi.cpp)
    target_include_directories(mark-shot-recording-bad-abi PRIVATE src)
    target_link_libraries(mark-shot-recording-bad-abi PRIVATE Qt6::Core Qt6::Gui)

    qt_add_executable(mark-shot-recording-backend-test
        tests/recording_backend_test.cpp
        ${MARK_SHOT_BACKEND_LOADER_SOURCES}
        src/recording/recording_async_writer.cpp
        src/recording/recording_async_writer.h
        src/recording/recording_frame_payload.cpp
        src/recording/recording_frame_queue.cpp
        src/recording/recording_memory_policy.cpp
        src/debug_log.cpp)
    target_include_directories(mark-shot-recording-backend-test PRIVATE src)
    target_compile_definitions(mark-shot-recording-backend-test PRIVATE
        MARK_SHOT_BAD_BACKEND_PATH="$<TARGET_FILE:mark-shot-recording-bad-abi>")
    if(NOT WIN32)
        target_compile_definitions(mark-shot-recording-backend-test PRIVATE
            MARK_SHOT_RECORDING_INSTALL_DIR="${_recording_install_dir}")
    endif()
    target_link_libraries(mark-shot-recording-backend-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
    set_target_properties(mark-shot-recording-backend-test PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY "${MARK_SHOT_RUNTIME_DIR}")
    add_dependencies(mark-shot-recording-backend-test mark-shot-recording-libav mark-shot-recording-bad-abi)
    add_test(NAME recording-backend COMMAND mark-shot-recording-backend-test)
    set_tests_properties(recording-backend PROPERTIES TIMEOUT 30)

    qt_add_executable(mark-shot-recording-video-timeline-test
        tests/recording_video_timeline_test.cpp
        src/recording/recording_frame_handoff.cpp
        src/recording/recording_frame_handoff.h
        src/recording/recording_frame_payload.cpp
        ${MARK_SHOT_BACKEND_LOADER_SOURCES}
        src/debug_log.cpp)
    target_include_directories(mark-shot-recording-video-timeline-test PRIVATE src)
    target_link_libraries(mark-shot-recording-video-timeline-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
    set_target_properties(mark-shot-recording-video-timeline-test PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY "${MARK_SHOT_RUNTIME_DIR}")
    add_dependencies(mark-shot-recording-video-timeline-test mark-shot-recording-libav)
    add_test(NAME recording-video-timeline COMMAND mark-shot-recording-video-timeline-test)
    set_tests_properties(recording-video-timeline PROPERTIES TIMEOUT 60)
endif()

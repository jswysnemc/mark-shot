if(MARK_SHOT_LINUX AND WaylandClient_FOUND AND WAYLAND_SCANNER_EXECUTABLE)
    find_package(Python3 QUIET COMPONENTS Interpreter)
    if(Python3_Interpreter_FOUND)
        qt_add_executable(mark-shot-recording-wlroots-buffer-pool-test
            tests/recording_wlroots_buffer_pool_test.cpp
            src/platform/wayland/wlroots_screencopy_buffer_pool.cpp
            src/platform/wayland/wlroots_screencopy_shm_buffer.cpp
            src/recording/recording_capture_memory_policy.cpp
            src/recording/recording_frame_payload.cpp)
        target_include_directories(mark-shot-recording-wlroots-buffer-pool-test PRIVATE src)
        target_compile_definitions(mark-shot-recording-wlroots-buffer-pool-test PRIVATE HAVE_WLROOTS_SCREENCOPY)
        target_link_libraries(mark-shot-recording-wlroots-buffer-pool-test PRIVATE
            Qt6::Gui Qt6::Test PkgConfig::WaylandClient)
        add_test(NAME recording-wlroots-buffer-pool
            COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/recording_wayland_session_test.py
                $<TARGET_FILE:mark-shot-recording-wlroots-buffer-pool-test>)
        set_tests_properties(recording-wlroots-buffer-pool PROPERTIES
            TIMEOUT 90 SKIP_RETURN_CODE 77 LABELS "wayland;integration")
        qt_add_executable(mark-shot-recording-wlroots-capture-test
            tests/recording_wlroots_capture_test.cpp
            src/platform/wayland/wlroots_screencopy_capture_stream.cpp
            src/platform/wayland/wlroots_screencopy_buffer_pool.cpp
            src/platform/wayland/wlroots_screencopy_shm_buffer.cpp
            src/platform/wayland/wlroots_screencopy_worker.h
            src/platform/wayland/wlroots_screencopy_worker_frame.cpp
            src/platform/wayland/wlroots_screencopy_worker_lifecycle.cpp
            src/platform/wayland/wlroots_screencopy_worker_registry.cpp
            src/recording/recording_capture_stream.h
            src/recording/recording_frame_pacer.cpp
            src/recording/recording_capture_memory_policy.cpp
            src/recording/recording_frame_payload.cpp
            src/recording/recording_frame_handoff.cpp
            src/recording/recording_frame_handoff.h
            src/debug_log.cpp
            "${MARK_SHOT_WLR_SCREENCOPY_CODE}")
        target_include_directories(mark-shot-recording-wlroots-capture-test PRIVATE
            src "${MARK_SHOT_WAYLAND_GENERATED_DIR}")
        target_compile_definitions(mark-shot-recording-wlroots-capture-test PRIVATE HAVE_WLROOTS_SCREENCOPY)
        target_link_libraries(mark-shot-recording-wlroots-capture-test PRIVATE
            Qt6::Gui Qt6::Test PkgConfig::WaylandClient)
        add_test(NAME recording-wlroots-capture
            COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/recording_wayland_session_test.py
                $<TARGET_FILE:mark-shot-recording-wlroots-capture-test>)
        set_tests_properties(recording-wlroots-capture PROPERTIES
            TIMEOUT 90 SKIP_RETURN_CODE 77 LABELS "wayland;integration")
    endif()
endif()

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

qt_add_executable(mark-shot-recording-bgra-buffer-pool-test
    tests/recording_bgra_buffer_pool_test.cpp
    src/recording/recording_bgra_buffer_pool.cpp
    src/recording/recording_bgra_buffer_pool.h
)
target_include_directories(mark-shot-recording-bgra-buffer-pool-test PRIVATE src)
target_link_libraries(mark-shot-recording-bgra-buffer-pool-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME recording-bgra-buffer-pool COMMAND mark-shot-recording-bgra-buffer-pool-test)

qt_add_executable(mark-shot-recording-dialog-config-test
    tests/recording_dialog_config_test.cpp
    src/recording/recording_dialog_config.cpp
    src/recording/recording_dialog_config.h
    src/recording/recording_capture_backend.cpp
    src/recording/recording_capture_backend.h
    src/recording/recording_file_naming.cpp
    src/recording/recording_file_naming.h
    src/recording/recording_storage_config.cpp
    src/recording/recording_storage_config.h
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
target_include_directories(mark-shot-recording-dialog-config-test PRIVATE src)
target_link_libraries(mark-shot-recording-dialog-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME recording-dialog-config COMMAND mark-shot-recording-dialog-config-test)

if(FFmpegLibav_FOUND)
    qt_add_executable(mark-shot-libav-recording-process-test
        tests/libav_recording_process_test.cpp
        src/recording/audio/audio_capture_reader.h
        src/recording/audio/audio_capture_reader_factory.cpp
        src/recording/audio/audio_capture_reader_factory.h
        src/recording/audio/audio_capture_sample.h
        src/recording/audio/pulse_audio_capture_reader.cpp
        src/recording/audio/pulse_audio_capture_reader.h
        src/recording/audio/wasapi_audio_capture_reader.cpp
        src/recording/audio/wasapi_audio_capture_reader.h
        src/debug_log.cpp
        src/debug_log.h
        src/recording/libav_audio_encoder.cpp
        src/recording/libav_audio_encoder.h
        src/recording/libav_error.cpp
        src/recording/libav_error.h
        src/recording/libav_gif_recording_process.cpp
        src/recording/libav_gif_recording_process.h
        src/recording/libav_recording_process.cpp
        src/recording/libav_recording_process.h
        src/recording/recording_frame_converter.cpp
        src/recording/recording_frame_converter.h
        src/recording/recording_frame_payload.cpp
        src/recording/recording_frame_payload.h
    )
    target_include_directories(mark-shot-libav-recording-process-test PRIVATE src)
    target_compile_definitions(mark-shot-libav-recording-process-test PRIVATE HAVE_LIBAV_RECORDING)
    if(PulseAudioRecording_FOUND)
        target_compile_definitions(mark-shot-libav-recording-process-test PRIVATE HAVE_PULSE_RECORDING)
    endif()
    target_link_libraries(mark-shot-libav-recording-process-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
            MarkShot::FFmpegLibav
    )
    if(PulseAudioRecording_FOUND)
        target_link_libraries(mark-shot-libav-recording-process-test PRIVATE PkgConfig::PulseAudioRecording)
    endif()
    if(WIN32)
        target_link_libraries(mark-shot-libav-recording-process-test PRIVATE ksuser ole32 uuid)
    endif()
    add_test(NAME libav-recording-process COMMAND mark-shot-libav-recording-process-test)
endif()

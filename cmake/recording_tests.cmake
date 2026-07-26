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

qt_add_executable(mark-shot-recording-frame-grabber-test
    tests/recording_frame_grabber_test.cpp
    src/debug_log.cpp
    src/debug_log.h
    src/recording/recording_capture_backend.cpp
    src/recording/recording_capture_backend.h
    src/recording/recording_frame_grabber.cpp
    src/recording/recording_frame_grabber.h
    src/recording/recording_capture_stream.h
)
target_include_directories(mark-shot-recording-frame-grabber-test PRIVATE src)
target_link_libraries(mark-shot-recording-frame-grabber-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME recording-frame-grabber COMMAND mark-shot-recording-frame-grabber-test)

qt_add_executable(mark-shot-recording-video-encoder-options-test
    tests/recording_video_encoder_options_test.cpp
    src/recording/recording_container_format.cpp
    src/recording/recording_container_format.h
    src/recording/recording_encoder_probe.cpp
    src/recording/recording_encoder_probe.h
    src/recording/recording_quality_options.cpp
    src/recording/recording_quality_options.h
    src/recording/recording_video_encoder_options.cpp
    src/recording/recording_video_encoder_options.h
)
target_include_directories(mark-shot-recording-video-encoder-options-test PRIVATE src)
target_link_libraries(mark-shot-recording-video-encoder-options-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME recording-video-encoder-options COMMAND mark-shot-recording-video-encoder-options-test)

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

if(MARK_SHOT_LINUX AND PipeWire_FOUND)
    qt_add_executable(mark-shot-pipewire-buffer-data-types-test
        tests/pipewire_buffer_data_types_test.cpp
        src/pipewire/pipewire_buffer_data_types.cpp
        src/pipewire/pipewire_buffer_data_types.h
        src/pipewire/pipewire_dmabuf_policy.cpp
        src/pipewire/pipewire_dmabuf_policy.h
        src/pipewire/pipewire_drm_fourcc.h
    )
    target_include_directories(mark-shot-pipewire-buffer-data-types-test PRIVATE src)
    target_link_libraries(mark-shot-pipewire-buffer-data-types-test
        PRIVATE
            Qt6::Core
            Qt6::Test
            PkgConfig::PipeWire
    )
    add_test(NAME pipewire-buffer-data-types COMMAND mark-shot-pipewire-buffer-data-types-test)
endif()

qt_add_executable(mark-shot-recording-dialog-config-test
    tests/recording_dialog_config_test.cpp
    src/recording/recording_dialog_config.cpp
    src/recording/recording_dialog_config.h
    src/recording/recording_capture_backend.cpp
    src/recording/recording_container_format.cpp
    src/recording/recording_container_format.h
    src/recording/recording_quality_options.cpp
    src/recording/recording_quality_options.h
    src/recording/recording_capture_backend.h
    src/recording/recording_file_naming.cpp
    src/recording/recording_file_naming.h
    src/recording/recording_storage_config.cpp
    src/recording/recording_storage_config.h
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
target_include_directories(mark-shot-recording-dialog-config-test PRIVATE src)
target_link_libraries(mark-shot-recording-dialog-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME recording-dialog-config COMMAND mark-shot-recording-dialog-config-test)

if(FFmpegLibav_FOUND)
    qt_add_executable(mark-shot-libav-video-scaler-test
        tests/libav_video_scaler_test.cpp
        src/recording/libav/libav_video_scaler.cpp
        src/recording/libav/libav_video_scaler.h
    )
    target_include_directories(mark-shot-libav-video-scaler-test PRIVATE src)
    target_compile_definitions(mark-shot-libav-video-scaler-test PRIVATE HAVE_LIBAV_RECORDING)
    target_link_libraries(mark-shot-libav-video-scaler-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
            MarkShot::FFmpegLibav
    )
    add_test(NAME libav-video-scaler COMMAND mark-shot-libav-video-scaler-test)

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
        src/recording/libav/libav_gif_palette_filter.cpp
        src/recording/libav/libav_gif_palette_filter.h
        src/recording/libav/libav_hw_encoder_context.cpp
        src/recording/libav/libav_hw_encoder_context.h
        src/recording/libav/libav_muxer.cpp
        src/recording/libav/libav_muxer.h
        src/recording/libav/libav_video_encoder_setup.cpp
        src/recording/libav/libav_video_encoder_setup.h
        src/recording/libav/libav_video_scaler.cpp
        src/recording/libav/libav_video_scaler.h
        src/recording/libav_gif_recording_process.cpp
        src/recording/libav_gif_recording_process.h
        src/recording/libav_recording_process.cpp
        src/recording/libav_recording_process.h
        src/recording/recording_container_format.cpp
        src/recording/recording_container_format.h
        src/recording/recording_encoder_probe.cpp
        src/recording/recording_encoder_probe.h
        src/recording/recording_quality_options.cpp
        src/recording/recording_quality_options.h
        src/recording/recording_frame_converter.cpp
        src/recording/recording_frame_converter.h
        src/recording/recording_frame_payload.cpp
        src/recording/recording_frame_payload.h
    )
    target_include_directories(mark-shot-libav-recording-process-test PRIVATE src)
    target_compile_definitions(mark-shot-libav-recording-process-test PRIVATE HAVE_LIBAV_RECORDING)
    if(FFmpegAvfilter_FOUND)
        target_compile_definitions(mark-shot-libav-recording-process-test PRIVATE HAVE_LIBAVFILTER)
        target_link_libraries(mark-shot-libav-recording-process-test PRIVATE MarkShot::FFmpegAvfilter)
    endif()
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

qt_add_executable(mark-shot-recording-pause-state-test
    tests/recording_pause_state_test.cpp
    src/recording/recording_pause_state.cpp
    src/recording/recording_pause_state.h
)
target_include_directories(mark-shot-recording-pause-state-test PRIVATE src)
target_link_libraries(mark-shot-recording-pause-state-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME recording-pause-state COMMAND mark-shot-recording-pause-state-test)

qt_add_executable(mark-shot-recording-frame-rate-limiter-test
    tests/recording_frame_rate_limiter_test.cpp
    src/recording/recording_frame_rate_limiter.cpp
    src/recording/recording_frame_rate_limiter.h
)
target_include_directories(mark-shot-recording-frame-rate-limiter-test PRIVATE src)
target_link_libraries(mark-shot-recording-frame-rate-limiter-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME recording-frame-rate-limiter COMMAND mark-shot-recording-frame-rate-limiter-test)

qt_add_executable(mark-shot-recording-overlay-layout-test
    tests/recording_overlay_layout_test.cpp
    src/recording/ui/recording_overlay_layout.cpp
    src/recording/ui/recording_overlay_layout.h
)
target_include_directories(mark-shot-recording-overlay-layout-test PRIVATE src)
target_link_libraries(mark-shot-recording-overlay-layout-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME recording-overlay-layout COMMAND mark-shot-recording-overlay-layout-test)

qt_add_executable(mark-shot-recording-frame-heartbeat-test
    tests/recording_frame_heartbeat_test.cpp
    src/recording/recording_frame_heartbeat.cpp
    src/recording/recording_frame_heartbeat.h
)
target_include_directories(mark-shot-recording-frame-heartbeat-test PRIVATE src)
target_link_libraries(mark-shot-recording-frame-heartbeat-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME recording-frame-heartbeat COMMAND mark-shot-recording-frame-heartbeat-test)

qt_add_executable(mark-shot-pipewire-dmabuf-policy-test
    tests/pipewire_dmabuf_policy_test.cpp
    src/pipewire/pipewire_dmabuf_policy.cpp
    src/pipewire/pipewire_dmabuf_policy.h
)
target_include_directories(mark-shot-pipewire-dmabuf-policy-test PRIVATE src)
target_link_libraries(mark-shot-pipewire-dmabuf-policy-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME pipewire-dmabuf-policy COMMAND mark-shot-pipewire-dmabuf-policy-test)

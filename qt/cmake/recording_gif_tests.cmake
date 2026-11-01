if(FFmpegLibav_FOUND)
    qt_add_executable(mark-shot-libav-gif-frame-cache-test
        tests/libav_gif_frame_cache_test.cpp
        src/recording/libav/libav_gif_frame_cache.cpp)
    target_include_directories(mark-shot-libav-gif-frame-cache-test PRIVATE src)
    target_compile_definitions(mark-shot-libav-gif-frame-cache-test PRIVATE HAVE_LIBAV_RECORDING)
    target_link_libraries(mark-shot-libav-gif-frame-cache-test PRIVATE
        Qt6::Core Qt6::Test MarkShot::FFmpegLibav)
    add_test(NAME libav-gif-frame-cache COMMAND mark-shot-libav-gif-frame-cache-test)

    set(_gif_repeat_sources
        tests/recording_gif_repeat_test.cpp
        src/recording/gif_recording_writer.cpp
        src/recording/libav_gif_recording_process.cpp
        src/recording/libav/libav_gif_frame_cache.cpp
        src/recording/libav/libav_gif_palette_filter.cpp
        src/recording/libav/libav_muxer.cpp
        src/recording/libav/libav_video_scaler.cpp
        src/recording/libav_error.cpp
        src/recording/recording_frame_converter.cpp
        src/recording/recording_frame_payload.cpp
        src/recording/recording_frame_rate_limiter.cpp)
    set(_gif_repeat_variants recording-gif-repeat)
    if(FFmpegAvfilter_FOUND)
        list(APPEND _gif_repeat_variants recording-gif-repeat-fixed-palette)
    endif()
    foreach(_variant IN LISTS _gif_repeat_variants)
        set(_target "mark-shot-${_variant}-test")
        qt_add_executable(${_target} ${_gif_repeat_sources})
        target_include_directories(${_target} PRIVATE src)
        target_compile_definitions(${_target} PRIVATE HAVE_LIBAV_RECORDING)
        target_link_libraries(${_target} PRIVATE Qt6::Core Qt6::Gui Qt6::Test MarkShot::FFmpegLibav)
        if(FFmpegAvfilter_FOUND AND _variant STREQUAL "recording-gif-repeat")
            target_compile_definitions(${_target} PRIVATE HAVE_LIBAVFILTER)
            target_link_libraries(${_target} PRIVATE MarkShot::FFmpegAvfilter)
        endif()
        add_test(NAME ${_variant} COMMAND ${_target})
        set_tests_properties(${_variant} PROPERTIES TIMEOUT 60)
    endforeach()
endif()

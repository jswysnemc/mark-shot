set(FFmpegLibav_FOUND FALSE)
set(FFmpegLibav_LIBRARIES)
set(FFmpegLibav_REQUIRED_VARS)
# libavfilter 仅用于 GIF 逐帧调色板量化，缺失时 GIF 回退固定调色板
set(FFmpegAvfilter_FOUND FALSE)

if(MARK_SHOT_LINUX)
    find_package(PkgConfig QUIET)
    if(PkgConfig_FOUND)
        pkg_check_modules(FFmpegLibav QUIET IMPORTED_TARGET
            libavcodec
            libavformat
            libavutil
            libswresample
            libswscale
        )
        if(FFmpegLibav_FOUND AND NOT TARGET MarkShot::FFmpegLibav)
            add_library(MarkShot::FFmpegLibav INTERFACE IMPORTED)
            target_link_libraries(MarkShot::FFmpegLibav INTERFACE PkgConfig::FFmpegLibav)
        endif()
        if(FFmpegLibav_FOUND)
            pkg_check_modules(FFmpegAvfilter QUIET IMPORTED_TARGET libavfilter)
            if(FFmpegAvfilter_FOUND AND NOT TARGET MarkShot::FFmpegAvfilter)
                add_library(MarkShot::FFmpegAvfilter INTERFACE IMPORTED)
                target_link_libraries(MarkShot::FFmpegAvfilter INTERFACE PkgConfig::FFmpegAvfilter)
            endif()
        endif()
    endif()
elseif(WIN32)
    include(FindPackageHandleStandardArgs)

    find_path(FFmpegLibav_INCLUDE_DIR
        NAMES libavcodec/avcodec.h
        HINTS ENV MINGW_PREFIX
        PATH_SUFFIXES include
    )

    foreach(component IN ITEMS avcodec avformat avutil swresample swscale)
        find_library(FFmpegLibav_${component}_LIBRARY
            NAMES ${component} lib${component}
            HINTS ENV MINGW_PREFIX
            PATH_SUFFIXES lib
        )
        list(APPEND FFmpegLibav_REQUIRED_VARS FFmpegLibav_${component}_LIBRARY)
        list(APPEND FFmpegLibav_LIBRARIES "${FFmpegLibav_${component}_LIBRARY}")
    endforeach()

    list(APPEND FFmpegLibav_REQUIRED_VARS FFmpegLibav_INCLUDE_DIR)
    find_package_handle_standard_args(FFmpegLibav
        REQUIRED_VARS ${FFmpegLibav_REQUIRED_VARS}
    )

    if(FFmpegLibav_FOUND AND NOT TARGET MarkShot::FFmpegLibav)
        add_library(MarkShot::FFmpegLibav INTERFACE IMPORTED)
        target_include_directories(MarkShot::FFmpegLibav INTERFACE "${FFmpegLibav_INCLUDE_DIR}")
        target_link_libraries(MarkShot::FFmpegLibav INTERFACE ${FFmpegLibav_LIBRARIES})
    endif()

    if(FFmpegLibav_FOUND)
        find_library(FFmpegLibav_avfilter_LIBRARY
            NAMES avfilter libavfilter
            HINTS ENV MINGW_PREFIX
            PATH_SUFFIXES lib
        )
        if(FFmpegLibav_avfilter_LIBRARY)
            set(FFmpegAvfilter_FOUND TRUE)
            if(NOT TARGET MarkShot::FFmpegAvfilter)
                add_library(MarkShot::FFmpegAvfilter INTERFACE IMPORTED)
                target_link_libraries(MarkShot::FFmpegAvfilter INTERFACE "${FFmpegLibav_avfilter_LIBRARY}")
            endif()
        endif()
    endif()
endif()

if(MARK_SHOT_REQUIRE_FFMPEG AND NOT FFmpegLibav_FOUND)
    message(FATAL_ERROR "FFmpeg libav support was required but libavcodec/libavformat/libavutil/libswresample/libswscale dependencies were not found")
endif()

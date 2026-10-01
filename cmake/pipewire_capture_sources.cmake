set(MARK_SHOT_PIPEWIRE_CAPTURE_SOURCES
    src/portal/screencast_restore_store.cpp
    src/portal/screencast_restore_store.h
    src/portal/screencast_persistence.cpp
    src/portal/screencast_persistence.h
    src/pipewire/pipewire_dmabuf_importer.cpp
    src/pipewire/pipewire_dmabuf_importer.h
    src/pipewire/pipewire_dmabuf_importer_gl_helpers.h
    src/screen_capture_pipewire.cpp
    src/screen_capture_pipewire_libportal.cpp
    src/screen_capture_pipewire_libportal.h
    src/screen_capture_pipewire_raw_frame.cpp
    src/screen_capture_pipewire_screencast.cpp
    src/screen_capture_pipewire_portal_dbus.cpp
    src/screen_capture_pipewire_portal_libportal.cpp
    src/screen_capture_pipewire_screencast.h
    src/screen_capture_pipewire_stream.cpp
)

if(MARK_SHOT_LINUX)
    list(INSERT MARK_SHOT_PIPEWIRE_CAPTURE_SOURCES 0
        src/pipewire/pipewire_buffer_data_types.cpp
        src/pipewire/pipewire_buffer_data_types.h
        src/pipewire/pipewire_dmabuf_policy.cpp
        src/pipewire/pipewire_dmabuf_policy.h
    )
endif()

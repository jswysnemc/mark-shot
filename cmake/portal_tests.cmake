qt_add_executable(mark-shot-screencast-restore-store-test
    tests/screencast_restore_store_test.cpp
    src/portal/screencast_restore_store.cpp
)
target_include_directories(mark-shot-screencast-restore-store-test PRIVATE src)
target_link_libraries(mark-shot-screencast-restore-store-test PRIVATE Qt6::Core Qt6::Test)
add_test(NAME screencast-restore-store COMMAND mark-shot-screencast-restore-store-test)

qt_add_executable(mark-shot-screencast-persistence-test
    tests/screencast_persistence_test.cpp
    src/portal/screencast_persistence.cpp
    src/portal/screencast_restore_store.cpp
    src/debug_log.cpp
)
target_include_directories(mark-shot-screencast-persistence-test PRIVATE src)
target_link_libraries(mark-shot-screencast-persistence-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME screencast-persistence COMMAND mark-shot-screencast-persistence-test)

if(MARK_SHOT_LINUX AND PipeWire_FOUND)
    qt_add_executable(mark-shot-screencast-authorization-test
        tests/screencast_authorization_test.cpp
        src/screen_capture_pipewire_screencast.cpp
        src/screen_capture_portal_guard.cpp
        src/portal/screencast_persistence.cpp
        src/portal/screencast_restore_store.cpp
        src/capture_geometry.cpp
        src/pipewire/pipewire_dmabuf_importer.cpp
        src/debug_log.cpp
    )
    target_include_directories(mark-shot-screencast-authorization-test PRIVATE src)
    target_compile_definitions(mark-shot-screencast-authorization-test PRIVATE HAVE_PIPEWIRE MARK_SHOT_WITH_DBUS=1)
    target_link_libraries(mark-shot-screencast-authorization-test
        PRIVATE Qt6::Core Qt6::Gui Qt6::Widgets Qt6::DBus Qt6::Test PkgConfig::PipeWire)
    if(MARK_SHOT_WITH_LIBPORTAL AND LibPortal_FOUND)
        target_compile_definitions(mark-shot-screencast-authorization-test PRIVATE HAVE_LIBPORTAL)
        target_link_libraries(mark-shot-screencast-authorization-test PRIVATE PkgConfig::LibPortal)
    endif()
    add_test(NAME screencast-authorization COMMAND mark-shot-screencast-authorization-test)
endif()

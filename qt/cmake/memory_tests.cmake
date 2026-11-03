qt_add_executable(mark-shot-idle-memory-trimmer-test
    tests/idle_memory_trimmer_test.cpp
    ${MARK_SHOT_MEMORY_SOURCES}
    src/debug_log.cpp
    src/debug_log.h
)
target_include_directories(mark-shot-idle-memory-trimmer-test PRIVATE src)
target_link_libraries(mark-shot-idle-memory-trimmer-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
if(WIN32)
    target_link_libraries(mark-shot-idle-memory-trimmer-test PRIVATE psapi)
endif()
add_test(NAME idle-memory-trimmer COMMAND mark-shot-idle-memory-trimmer-test)
set_tests_properties(idle-memory-trimmer PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(mark-shot-region-image-memory-test
    tests/region_image_memory_test.cpp
    src/region_detection/image_gray.cpp)
target_include_directories(mark-shot-region-image-memory-test PRIVATE src)
target_link_libraries(mark-shot-region-image-memory-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME region-image-memory COMMAND mark-shot-region-image-memory-test)

qt_add_executable(mark-shot-canvas-image-input-test
    tests/canvas_image_input_test.cpp
    src/canvas/canvas_image_input.cpp)
target_include_directories(mark-shot-canvas-image-input-test PRIVATE src)
target_link_libraries(mark-shot-canvas-image-input-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME canvas-image-input COMMAND mark-shot-canvas-image-input-test)

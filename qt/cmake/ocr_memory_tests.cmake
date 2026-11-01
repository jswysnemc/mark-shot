qt_add_executable(mark-shot-ocr-plugin-task-lifecycle-test
    tests/ocr_plugin_task_lifecycle_test.cpp
    src/providers/ocr/ocr_task_executor.cpp
    src/providers/ocr/ocr_plugin_task.cpp
    src/providers/provider_task.cpp
    src/memory/process_memory_trim.cpp
    src/debug_log.cpp)
target_include_directories(mark-shot-ocr-plugin-task-lifecycle-test PRIVATE src plugin-sdk)
target_link_libraries(mark-shot-ocr-plugin-task-lifecycle-test PRIVATE
    Qt6::Core Qt6::Gui Qt6::Concurrent Qt6::Test)
if(WIN32)
    target_link_libraries(mark-shot-ocr-plugin-task-lifecycle-test PRIVATE psapi)
endif()
add_test(NAME ocr-plugin-task-lifecycle COMMAND mark-shot-ocr-plugin-task-lifecycle-test)
set_tests_properties(ocr-plugin-task-lifecycle PROPERTIES TIMEOUT 30)

qt_add_executable(mark-shot-rapid-image-input-test
    tests/rapid_image_input_test.cpp
    plugins/ocr-rapid/rapid_image_input.cpp)
target_include_directories(mark-shot-rapid-image-input-test PRIVATE plugins/ocr-rapid)
target_link_libraries(mark-shot-rapid-image-input-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME rapid-image-input COMMAND mark-shot-rapid-image-input-test)

include(cmake/ocr_runtime_tests.cmake)

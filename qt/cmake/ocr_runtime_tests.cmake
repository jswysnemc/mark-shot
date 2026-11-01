if(NOT TARGET mark-shot-onnx-headers)
    return()
endif()

add_library(mark-shot-onnx-lifetime-fixture SHARED tests/fixtures/onnx_runtime_lifetime.cpp)
target_compile_definitions(mark-shot-onnx-lifetime-fixture PRIVATE ORT_BUILD_SHARED_LIB)
target_link_libraries(mark-shot-onnx-lifetime-fixture PRIVATE mark-shot-onnx-headers)
set_target_properties(mark-shot-onnx-lifetime-fixture PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/fixtures/onnx-lifetime"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/fixtures/onnx-lifetime")

add_library(mark-shot-onnx-incompatible-fixture SHARED tests/fixtures/onnx_runtime_conflict.c)
set_target_properties(mark-shot-onnx-incompatible-fixture PROPERTIES WINDOWS_EXPORT_ALL_SYMBOLS ON
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/fixtures/onnx-incompatible"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/fixtures/onnx-incompatible")

qt_add_executable(mark-shot-rapid-onnx-runtime-test
    tests/rapid_onnx_runtime_test.cpp
    plugins/ocr-rapid/rapid_onnx_runtime.cpp
    plugins/ocr-rapid/rapid_runtime_paths.cpp)
target_include_directories(mark-shot-rapid-onnx-runtime-test PRIVATE plugins/ocr-rapid)
target_link_libraries(mark-shot-rapid-onnx-runtime-test PRIVATE
    Qt6::Core Qt6::Test mark-shot-onnx-headers ${CMAKE_DL_LIBS})
target_compile_definitions(mark-shot-rapid-onnx-runtime-test PRIVATE
    MARK_SHOT_TEST_ORT_LIBRARY="$<TARGET_FILE:mark-shot-onnx-lifetime-fixture>"
    MARK_SHOT_TEST_ORT_INCOMPATIBLE="$<TARGET_FILE:mark-shot-onnx-incompatible-fixture>")
add_dependencies(mark-shot-rapid-onnx-runtime-test
    mark-shot-onnx-lifetime-fixture mark-shot-onnx-incompatible-fixture)
add_test(NAME rapid-onnx-runtime COMMAND mark-shot-rapid-onnx-runtime-test)
set_tests_properties(rapid-onnx-runtime PROPERTIES TIMEOUT 30)

qt_add_executable(mark-shot-pinned-ocr-memory-test
    tests/pinned_ocr_memory_test.cpp
    plugins/ocr-rapid/rapid_model_paths.cpp
    "$<FILTER:$<TARGET_OBJECTS:mark-shot>,EXCLUDE,/main\\.cpp\\.(o|obj)$>")
target_include_directories(mark-shot-pinned-ocr-memory-test PRIVATE tests plugin-sdk plugins/ocr-rapid
    "$<TARGET_PROPERTY:mark-shot,INCLUDE_DIRECTORIES>")
target_compile_definitions(mark-shot-pinned-ocr-memory-test PRIVATE
    "$<TARGET_PROPERTY:mark-shot,COMPILE_DEFINITIONS>"
    MARK_SHOT_TEST_OCR_PLUGIN_PATH="$<TARGET_FILE:mark-shot-ocr-rapid>")
target_link_libraries(mark-shot-pinned-ocr-memory-test PRIVATE
    "$<TARGET_PROPERTY:mark-shot,LINK_LIBRARIES>" Qt6::Test)
get_target_property(ocr_window_standard mark-shot CXX_STANDARD)
set_target_properties(mark-shot-pinned-ocr-memory-test PROPERTIES CXX_STANDARD ${ocr_window_standard})
if(WIN32)
    set_target_properties(mark-shot-pinned-ocr-memory-test PROPERTIES WIN32_EXECUTABLE TRUE)
    target_link_libraries(mark-shot-pinned-ocr-memory-test PRIVATE psapi)
endif()
add_dependencies(mark-shot-pinned-ocr-memory-test mark-shot mark-shot-ocr-rapid)
add_test(NAME pinned-ocr-memory COMMAND mark-shot-pinned-ocr-memory-test -o -,txt)
set_tests_properties(pinned-ocr-memory PROPERTIES TIMEOUT 90 ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(Python3 QUIET COMPONENTS Interpreter)
    if(Python3_Interpreter_FOUND)
        # 【贴图测试】【桌面隔离】复用录制测试的独立 D-Bus、Xvfb 与 niri 启动器，不采集桌面
        add_test(NAME pinned-ocr-wayland
            COMMAND ${CMAKE_COMMAND} -E env "MARK_SHOT_PIN_TEST_WAYLAND=1"
                ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/recording_wayland_session_test.py
                $<TARGET_FILE:mark-shot-pinned-ocr-memory-test>)
        set_tests_properties(pinned-ocr-wayland PROPERTIES
            TIMEOUT 90 SKIP_RETURN_CODE 77 LABELS "wayland;integration")
    endif()
    find_library(MARK_SHOT_TEST_PRIVATE_ONNX_RUNTIME NAMES onnxruntime)
    if(MARK_SHOT_TEST_PRIVATE_ONNX_RUNTIME)
        add_test(NAME pinned-ocr-private-runtime
            COMMAND ${CMAKE_COMMAND} -E env
                "MARK_SHOT_TEST_PRIVATE_RUNTIME=${MARK_SHOT_TEST_PRIVATE_ONNX_RUNTIME}"
                $<TARGET_FILE:mark-shot-pinned-ocr-memory-test> -o -,txt)
        set_tests_properties(pinned-ocr-private-runtime PROPERTIES TIMEOUT 90)
    endif()
endif()

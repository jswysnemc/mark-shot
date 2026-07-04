qt_add_executable(mark-shot-translate-segments-test
    tests/translate_segments_test.cpp
    src/providers/translate/translate_segments.cpp
    src/providers/translate/translate_segments.h
    src/ocr_result.cpp
    src/ocr_result.h
)
target_include_directories(mark-shot-translate-segments-test PRIVATE src)
target_link_libraries(mark-shot-translate-segments-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME translate-segments COMMAND mark-shot-translate-segments-test)

qt_add_executable(mark-shot-ocr-provider-factory-test
    tests/ocr_provider_factory_test.cpp
    src/debug_log.cpp
    src/debug_log.h
    src/providers/ocr/ocr_plugin_task.cpp
    src/providers/ocr/ocr_plugin_task.h
    src/providers/ocr/ocr_provider_factory.cpp
    src/providers/ocr/ocr_provider_factory.h
    src/providers/ocr/ocr_tesseract_task.cpp
    src/providers/ocr/ocr_tesseract_task.h
    src/providers/provider_plugin_registry.cpp
    src/providers/provider_plugin_registry.h
    src/providers/provider_process_task.cpp
    src/providers/provider_process_task.h
    src/providers/provider_task.cpp
    src/providers/provider_task.h
    src/shell_command.cpp
    src/shell_command.h
)
target_include_directories(mark-shot-ocr-provider-factory-test PRIVATE src plugin-sdk)
target_link_libraries(mark-shot-ocr-provider-factory-test
    PRIVATE
        Qt6::Core
        Qt6::Concurrent
        Qt6::Gui
        Qt6::Test
)
add_test(NAME ocr-provider-factory COMMAND mark-shot-ocr-provider-factory-test)

if(TARGET mark-shot-ocr-rapid)
    # pkg_check_modules 的 IMPORTED target 为目录作用域，测试目录内重新探测
    find_package(PkgConfig QUIET)
    if(PkgConfig_FOUND)
        pkg_check_modules(OnnxRuntimeTest QUIET IMPORTED_TARGET libonnxruntime)
    endif()
    if(NOT OnnxRuntimeTest_FOUND)
        find_package(onnxruntime CONFIG QUIET)
    endif()
endif()

if(TARGET mark-shot-ocr-rapid AND (OnnxRuntimeTest_FOUND OR onnxruntime_FOUND))
    qt_add_executable(mark-shot-ocr-rapid-plugin-test
        tests/ocr_rapid_plugin_test.cpp
        plugins/ocr-rapid/rapid_det_model.cpp
        plugins/ocr-rapid/rapid_det_model.h
        plugins/ocr-rapid/rapid_model_paths.cpp
        plugins/ocr-rapid/rapid_model_paths.h
        plugins/ocr-rapid/rapid_ocr_plugin.cpp
        plugins/ocr-rapid/rapid_ocr_plugin.h
        plugins/ocr-rapid/rapid_onnx_session.cpp
        plugins/ocr-rapid/rapid_onnx_session.h
        plugins/ocr-rapid/rapid_rec_model.cpp
        plugins/ocr-rapid/rapid_rec_model.h
    )
    target_include_directories(mark-shot-ocr-rapid-plugin-test PRIVATE
        plugins/ocr-rapid
        plugin-sdk
    )
    target_link_libraries(mark-shot-ocr-rapid-plugin-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
    )
    if(OnnxRuntimeTest_FOUND)
        target_link_libraries(mark-shot-ocr-rapid-plugin-test PRIVATE PkgConfig::OnnxRuntimeTest)
    else()
        target_link_libraries(mark-shot-ocr-rapid-plugin-test PRIVATE onnxruntime::onnxruntime)
    endif()
    add_test(NAME ocr-rapid-plugin COMMAND mark-shot-ocr-rapid-plugin-test)
endif()

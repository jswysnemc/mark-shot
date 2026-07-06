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
    src/providers/provider_plugin_paths.cpp
    src/providers/provider_plugin_paths.h
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

qt_add_executable(mark-shot-rapid-ocr-word-segments-test
    tests/rapid_ocr_word_segments_test.cpp
    plugins/ocr-rapid/rapid_ocr_word_segments.cpp
    plugins/ocr-rapid/rapid_ocr_word_segments.h
)
target_include_directories(mark-shot-rapid-ocr-word-segments-test PRIVATE
    plugins/ocr-rapid
)
target_link_libraries(mark-shot-rapid-ocr-word-segments-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME rapid-ocr-word-segments COMMAND mark-shot-rapid-ocr-word-segments-test)

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
        plugins/ocr-rapid/rapid_ocr_word_segments.cpp
        plugins/ocr-rapid/rapid_ocr_word_segments.h
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

if(TARGET mark-shot-translate-openai)
    qt_add_executable(mark-shot-translate-openai-plugin-test
        tests/translate_openai_plugin_test.cpp
        plugins/translate-openai/openai_translate_config.cpp
        plugins/translate-openai/openai_translate_config.h
        plugins/translate-openai/openai_translate_plugin.cpp
        plugins/translate-openai/openai_translate_plugin.h
        plugins/translate-openai/openai_translation_parser.cpp
        plugins/translate-openai/openai_translation_parser.h
    )
    target_include_directories(mark-shot-translate-openai-plugin-test PRIVATE
        plugins/translate-openai
        plugin-sdk
    )
    target_link_libraries(mark-shot-translate-openai-plugin-test
        PRIVATE
            Qt6::Core
            Qt6::Network
            Qt6::Test
    )
    add_test(NAME translate-openai-plugin COMMAND mark-shot-translate-openai-plugin-test)
endif()

if(TARGET mark-shot-code-scan-zxing)
    qt_add_executable(mark-shot-code-scan-zxing-plugin-test
        tests/code_scan_zxing_plugin_test.cpp
        plugins/code-scan-zxing/zxing_code_scan_plugin.cpp
        plugins/code-scan-zxing/zxing_code_scan_plugin.h
    )
    target_include_directories(mark-shot-code-scan-zxing-plugin-test PRIVATE
        plugins/code-scan-zxing
        plugin-sdk
    )
    target_link_libraries(mark-shot-code-scan-zxing-plugin-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
    )
    if(ZXing_FOUND)
        target_link_libraries(mark-shot-code-scan-zxing-plugin-test PRIVATE ZXing::ZXing)
    elseif(ZXingCpp_FOUND)
        target_link_libraries(mark-shot-code-scan-zxing-plugin-test PRIVATE PkgConfig::ZXingCpp)
    elseif(TARGET PkgConfig::ZXingPlugin)
        target_link_libraries(mark-shot-code-scan-zxing-plugin-test PRIVATE PkgConfig::ZXingPlugin)
    endif()
    add_test(NAME code-scan-zxing-plugin COMMAND mark-shot-code-scan-zxing-plugin-test)
endif()

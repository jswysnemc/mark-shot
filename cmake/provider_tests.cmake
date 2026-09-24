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
    src/marketplace/plugin_installer.cpp
    src/marketplace/plugin_updates.cpp
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

qt_add_executable(mark-shot-rapid-model-paths-test
    tests/rapid_model_paths_test.cpp
    plugins/ocr-rapid/rapid_model_paths.cpp
)
target_include_directories(mark-shot-rapid-model-paths-test PRIVATE plugins/ocr-rapid plugin-sdk)
target_link_libraries(mark-shot-rapid-model-paths-test PRIVATE Qt6::Core Qt6::Test)
add_test(NAME rapid-model-paths COMMAND mark-shot-rapid-model-paths-test -o -,txt)

if(TARGET mark-shot-ocr-rapid)
    # 1. 【OCR】【插件测试】测试进程仅链接 Qt，避免预加载依赖掩盖动态库加载错误
    qt_add_executable(mark-shot-ocr-rapid-plugin-test
        tests/ocr_rapid_plugin_test.cpp
        plugins/ocr-rapid/rapid_model_paths.cpp
        plugins/ocr-rapid/rapid_model_paths.h
        src/marketplace/plugin_installer.cpp
        src/marketplace/plugin_updates.cpp
        src/providers/provider_plugin_paths.cpp
    )
    target_include_directories(mark-shot-ocr-rapid-plugin-test PRIVATE
        plugins/ocr-rapid
        plugin-sdk
        src
    )
    target_link_libraries(mark-shot-ocr-rapid-plugin-test
        PRIVATE
            Qt6::Core
            Qt6::Gui
            Qt6::Test
    )
    target_compile_definitions(mark-shot-ocr-rapid-plugin-test PRIVATE
        MARK_SHOT_TEST_OCR_PLUGIN_PATH="$<TARGET_FILE:mark-shot-ocr-rapid>"
    )
    add_dependencies(mark-shot-ocr-rapid-plugin-test mark-shot-ocr-rapid)
    if(WIN32)
        # 2. 【OCR】【测试运行时】Windows 先搜索程序目录，再搜索系统目录和 PATH
        # 将构建所用的运行时放在测试入口旁，避免误加载 System32 中旧版 ONNX Runtime
        find_package(PkgConfig QUIET)
        if(PkgConfig_FOUND)
            pkg_check_modules(OcrTestRuntime QUIET libonnxruntime)
        endif()
        if(NOT OcrTestRuntime_FOUND)
            find_package(onnxruntime CONFIG QUIET)
        endif()
        set(ocr_runtime_hints "$ENV{MSYSTEM_PREFIX}/bin"
            "${onnxruntime_DIR}/../../bin" "${onnxruntime_DIR}/../../../bin")
        foreach(library_dir IN LISTS OcrTestRuntime_LIBRARY_DIRS)
            list(APPEND ocr_runtime_hints "${library_dir}/../bin")
        endforeach()
        find_file(MARK_SHOT_TEST_ONNX_RUNTIME_DLL NAMES onnxruntime.dll libonnxruntime.dll
            HINTS ${ocr_runtime_hints} NO_DEFAULT_PATH REQUIRED)
        add_custom_command(TARGET mark-shot-ocr-rapid-plugin-test POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${MARK_SHOT_TEST_ONNX_RUNTIME_DLL}"
                "$<TARGET_FILE_DIR:mark-shot-ocr-rapid-plugin-test>"
        )
    endif()
    add_test(NAME ocr-rapid-plugin COMMAND mark-shot-ocr-rapid-plugin-test -o -,txt)
    set_tests_properties(ocr-rapid-plugin PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
    if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
        add_test(NAME ocr-rapid-plugin-dependencies
            COMMAND ${CMAKE_COMMAND}
                "-DREADELF=${CMAKE_READELF}"
                "-DPLUGIN_PATH=$<TARGET_FILE:mark-shot-ocr-rapid>"
                -P "${CMAKE_SOURCE_DIR}/tests/ocr_plugin_dependencies.cmake"
        )
    endif()
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
        src/providers/provider_task.cpp
        src/providers/provider_task.h
        src/providers/translate/translate_openai_task.cpp
        src/providers/translate/translate_openai_task.h
        src/providers/translate/translate_segments.cpp
        src/ocr_result.cpp
    )
    target_include_directories(mark-shot-translate-openai-plugin-test PRIVATE
        plugins/translate-openai
        plugin-sdk
        src
    )
    target_link_libraries(mark-shot-translate-openai-plugin-test
        PRIVATE
            mark-shot-translate-common
            Qt6::Core
            Qt6::Gui
            Qt6::Network
            Qt6::Test
    )
    add_test(NAME translate-openai-plugin COMMAND mark-shot-translate-openai-plugin-test)
endif()

if(TARGET mark-shot-translate-tencent)
    qt_add_executable(mark-shot-translate-tencent-plugin-test
        tests/translate_tencent_plugin_test.cpp
        plugins/translate-tencent/tencent_tc3_signer.cpp
        plugins/translate-tencent/tencent_tc3_signer.h
        plugins/translate-tencent/tencent_translate_config.cpp
        plugins/translate-tencent/tencent_translate_config.h
        plugins/translate-tencent/tencent_translate_plugin.cpp
        plugins/translate-tencent/tencent_translate_plugin.h
        plugins/translate-tencent/tencent_translate_request.cpp
        plugins/translate-tencent/tencent_translate_request.h
    )
    target_include_directories(mark-shot-translate-tencent-plugin-test PRIVATE
        plugins/translate-tencent
        plugin-sdk
    )
    target_link_libraries(mark-shot-translate-tencent-plugin-test
        PRIVATE
            mark-shot-translate-common
            Qt6::Core
            Qt6::Network
            Qt6::Test
    )
    add_test(NAME translate-tencent-plugin COMMAND mark-shot-translate-tencent-plugin-test)
endif()

if(TARGET mark-shot-translate-baidu)
    qt_add_executable(mark-shot-translate-baidu-plugin-test
        tests/translate_baidu_plugin_test.cpp
        plugins/translate-baidu/baidu_translate_config.cpp
        plugins/translate-baidu/baidu_translate_config.h
        plugins/translate-baidu/baidu_translate_plugin.cpp
        plugins/translate-baidu/baidu_translate_plugin.h
        plugins/translate-baidu/baidu_translate_request.cpp
        plugins/translate-baidu/baidu_translate_request.h
        plugins/translate-baidu/baidu_translate_signer.cpp
        plugins/translate-baidu/baidu_translate_signer.h
    )
    target_include_directories(mark-shot-translate-baidu-plugin-test PRIVATE
        plugins/translate-baidu
        plugin-sdk
    )
    target_link_libraries(mark-shot-translate-baidu-plugin-test
        PRIVATE
            mark-shot-translate-common
            Qt6::Core
            Qt6::Network
            Qt6::Test
    )
    add_test(NAME translate-baidu-plugin COMMAND mark-shot-translate-baidu-plugin-test)
endif()

if(TARGET mark-shot-translate-youdao)
    qt_add_executable(mark-shot-translate-youdao-plugin-test
        tests/translate_youdao_plugin_test.cpp
        plugins/translate-youdao/youdao_translate_config.cpp
        plugins/translate-youdao/youdao_translate_config.h
        plugins/translate-youdao/youdao_translate_plugin.cpp
        plugins/translate-youdao/youdao_translate_plugin.h
        plugins/translate-youdao/youdao_translate_request.cpp
        plugins/translate-youdao/youdao_translate_request.h
        plugins/translate-youdao/youdao_translate_signer.cpp
        plugins/translate-youdao/youdao_translate_signer.h
    )
    target_include_directories(mark-shot-translate-youdao-plugin-test PRIVATE
        plugins/translate-youdao
        plugin-sdk
    )
    target_link_libraries(mark-shot-translate-youdao-plugin-test
        PRIVATE
            mark-shot-translate-common
            Qt6::Core
            Qt6::Network
            Qt6::Test
    )
    add_test(NAME translate-youdao-plugin COMMAND mark-shot-translate-youdao-plugin-test)
endif()

if(TARGET mark-shot-translate-gemini)
    qt_add_executable(mark-shot-translate-gemini-plugin-test
        tests/translate_gemini_plugin_test.cpp
        plugins/translate-gemini/gemini_translate_config.cpp
        plugins/translate-gemini/gemini_translate_config.h
        plugins/translate-gemini/gemini_translate_plugin.cpp
        plugins/translate-gemini/gemini_translate_plugin.h
        plugins/translate-gemini/gemini_translate_request.cpp
        plugins/translate-gemini/gemini_translate_request.h
    )
    target_include_directories(mark-shot-translate-gemini-plugin-test PRIVATE
        plugins/translate-gemini
        plugin-sdk
    )
    target_link_libraries(mark-shot-translate-gemini-plugin-test
        PRIVATE
            mark-shot-translate-common
            Qt6::Core
            Qt6::Network
            Qt6::Test
    )
    add_test(NAME translate-gemini-plugin COMMAND mark-shot-translate-gemini-plugin-test)
endif()

if(TARGET mark-shot-translate-anthropic)
    qt_add_executable(mark-shot-translate-anthropic-plugin-test
        tests/translate_anthropic_plugin_test.cpp
        plugins/translate-anthropic/anthropic_translate_config.cpp
        plugins/translate-anthropic/anthropic_translate_config.h
        plugins/translate-anthropic/anthropic_translate_plugin.cpp
        plugins/translate-anthropic/anthropic_translate_plugin.h
        plugins/translate-anthropic/anthropic_translate_request.cpp
        plugins/translate-anthropic/anthropic_translate_request.h
    )
    target_include_directories(mark-shot-translate-anthropic-plugin-test PRIVATE
        plugins/translate-anthropic
        plugin-sdk
    )
    target_link_libraries(mark-shot-translate-anthropic-plugin-test
        PRIVATE
            mark-shot-translate-common
            Qt6::Core
            Qt6::Network
            Qt6::Test
    )
    add_test(NAME translate-anthropic-plugin COMMAND mark-shot-translate-anthropic-plugin-test)
endif()

if(TARGET mark-shot-code-scan-zxing AND MARK_SHOT_ZXING_WRITER_SUPPORTED)
    qt_add_executable(mark-shot-code-scan-zxing-plugin-test
        tests/code_scan_zxing_plugin_test.cpp
        plugins/code-scan-zxing/zxing_code_scan_plugin.cpp
        plugins/code-scan-zxing/zxing_code_scan_plugin.h
    )
    target_include_directories(mark-shot-code-scan-zxing-plugin-test PRIVATE
        plugins/code-scan-zxing
        plugin-sdk
    )
    target_compile_definitions(mark-shot-code-scan-zxing-plugin-test PRIVATE ${MARK_SHOT_ZXING_COMPILE_DEFINITIONS})
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

# ocr-rapid 插件：PP-OCR ONNX 推理 provider，onnxruntime 缺失时整体跳过

find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
    pkg_check_modules(OnnxRuntime QUIET IMPORTED_TARGET libonnxruntime)
endif()
if(NOT OnnxRuntime_FOUND)
    find_package(onnxruntime CONFIG QUIET)
endif()

if(NOT OnnxRuntime_FOUND AND NOT onnxruntime_FOUND)
    message(STATUS "mark-shot: ocr-rapid plugin disabled (onnxruntime not found)")
    return()
endif()

qt_add_plugin(mark-shot-ocr-rapid
    SHARED
    rapid_det_model.cpp
    rapid_det_model.h
    rapid_model_paths.cpp
    rapid_model_paths.h
    rapid_ocr_plugin.cpp
    rapid_ocr_plugin.h
    rapid_ocr_word_segments.cpp
    rapid_ocr_word_segments.h
    rapid_onnx_session.cpp
    rapid_onnx_session.h
    rapid_rec_model.cpp
    rapid_rec_model.h
)

target_include_directories(mark-shot-ocr-rapid PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_SOURCE_DIR}/plugin-sdk
)
target_link_libraries(mark-shot-ocr-rapid
    PRIVATE
        Qt6::Core
        Qt6::Gui
)
if(OnnxRuntime_FOUND)
    target_link_libraries(mark-shot-ocr-rapid PRIVATE PkgConfig::OnnxRuntime)
else()
    target_link_libraries(mark-shot-ocr-rapid PRIVATE onnxruntime::onnxruntime)
endif()

install(TARGETS mark-shot-ocr-rapid
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins)

message(STATUS "mark-shot: ocr-rapid plugin enabled")
